use crate::function_tool::FunctionCallError;
use crate::session_hooks::ToolContinuation;
use crate::tools::context::ToolCallState;
use crate::tools::context::ToolInvocation;
use crate::tools::registry::AnyToolResult;
use codex_claude_mods_extension::Dispatch;
use codex_claude_mods_extension::ModResult;
use futures::future::BoxFuture;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

pub(crate) async fn dispatch<'call>(
    invocation: ToolInvocation,
    call_state: Option<Arc<ToolCallState>>,
    mut next: ToolContinuation<'call>,
) -> Result<AnyToolResult, FunctionCallError> {
    super::state(&invocation.session)
        .check()
        .map_err(FunctionCallError::RespondToModel)?;
    let Some(runtime) = super::state(&invocation.session).runtime() else {
        return next(invocation, call_state).await;
    };
    let events = super::tool_events::events(&invocation)
        .await
        .map_err(FunctionCallError::RespondToModel)?;
    let result = Arc::new(Mutex::new(None));
    let first_event_index = 0;
    let outcome = chain(
        &runtime,
        &events,
        first_event_index,
        invocation.clone(),
        call_state,
        Arc::clone(&result),
        Arc::new(std::sync::Mutex::new(next)),
    )
    .await;
    let mut native = result.lock().await.take();
    if matches!(native, Some(Err(_))) {
        return native.take().ok_or_else(|| {
            FunctionCallError::RespondToModel("native tool result disappeared".into())
        })?;
    }
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(message) => {
            if let Some(native) = native {
                report_after_execution(
                    &invocation,
                    format!("The native tool completed, but its mod hook failed: {message}"),
                )
                .await;
                return native;
            }
            return Err(FunctionCallError::RespondToModel(message));
        }
    };
    if let Some(reason) = outcome["deny"].as_str() {
        if let Some(native) = native {
            report_after_execution(
                &invocation,
                format!("The mod returned a denial after the native tool completed: {reason}"),
            )
            .await;
            return native;
        }
        return Err(FunctionCallError::RespondToModel(format!(
            "Claude mod denied the tool: {reason}"
        )));
    }
    if let Some(context) = outcome["context"].as_array() {
        let messages = context
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        crate::hook_runtime::record_additional_contexts(
            &invocation.session,
            &invocation.turn,
            messages,
        )
        .await;
    }
    if let Some(native) = native {
        return native;
    }
    let output = outcome.get("result").cloned().ok_or_else(|| {
        FunctionCallError::RespondToModel("a mod must return a tool result or call next".into())
    })?;
    Ok(AnyToolResult {
        call_id: invocation.call_id,
        payload: invocation.payload,
        result: Box::new(codex_tools::JsonToolOutput::with_success(
            output,
            Some(outcome["isError"].as_bool() != Some(true)),
        )),
        post_tool_use_payload: None,
    })
}

fn chain<'call, 'next: 'call>(
    runtime: &'call codex_claude_mods_extension::ModRuntime,
    events: &'call [Value],
    index: usize,
    invocation: ToolInvocation,
    call_state: Option<Arc<ToolCallState>>,
    result: Arc<Mutex<Option<Result<AnyToolResult, FunctionCallError>>>>,
    next: Arc<std::sync::Mutex<ToolContinuation<'next>>>,
) -> BoxFuture<'call, ModResult<Value>> {
    Box::pin(async move {
        let Some(event) = events.get(index) else {
            let continuation = next
                .lock()
                .map_err(|_| "tool continuation lock poisoned".to_string())?(
                invocation, call_state,
            );
            let native = match continuation.await {
                Ok(native) => native,
                Err(error) => {
                    let message = error.to_string();
                    *result.lock().await = Some(Err(error));
                    return Err(message);
                }
            };
            let output = native.result.code_mode_result(&native.payload);
            let successful = native.result.success_for_logging();
            *result.lock().await = Some(Ok(native));
            return Ok(
                json!({"isError":!successful,"result":output,"nativeResult":output,"context":[]}),
            );
        };
        runtime
            .dispatch(
                Dispatch {
                    name: "tool.call".into(),
                    input: event.clone(),
                },
                |changed| {
                    let next = Arc::clone(&next);
                    let mut next_invocation = invocation.clone();
                    let next_result = Arc::clone(&result);
                    let next_state = call_state.clone();
                    let result_invocation = invocation.clone();
                    let updated = super::tool_input::update(&mut next_invocation, event, &changed);
                    async move {
                        updated?;
                        let outcome = chain(
                            runtime,
                            events,
                            index + 1,
                            next_invocation,
                            next_state,
                            next_result,
                            next,
                        )
                        .await?;
                        Ok(super::tool_events::result(&result_invocation, event, outcome).await)
                    }
                },
            )
            .await
    })
}

async fn report_after_execution(invocation: &ToolInvocation, message: String) {
    invocation
        .session
        .send_event_raw(codex_protocol::protocol::Event {
            id: invocation.turn.sub_id.clone(),
            msg: codex_protocol::protocol::EventMsg::ClaudeMod(
                codex_protocol::claude_mods::ClaudeModEvent::Error {
                    plugin_id: "runtime".into(),
                    message,
                },
            ),
        })
        .await;
}
