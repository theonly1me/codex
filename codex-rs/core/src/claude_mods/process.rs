use super::host::step;
use super::host::string;
use crate::session::session::Session;
use crate::tools::context::ToolCallSource;
use crate::tools::context::ToolInvocation;
use crate::tools::handlers::unified_exec::ExecCommandHandler;
use crate::tools::handlers::unified_exec::ExecCommandHandlerOptions;
use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::ModResult;
use codex_tools::ToolExecutor;
use codex_tools::ToolName;
use codex_tools::ToolPayload;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let arguments = call
        .arguments
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| "process.run requires an argument array".to_string())?;
    if arguments.is_empty() {
        return Err("process.run requires a command".into());
    }
    let arguments = arguments
        .iter()
        .map(string)
        .collect::<ModResult<Vec<_>>>()?;
    let command = arguments
        .iter()
        .map(|argument| format!("'{}'", argument.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");
    let step = step(session).await?;
    let mod_home = step.turn.config.codex_home.join("claude-mods/home");
    let command = format!(
        "HOME='{}' {command}",
        mod_home.display().to_string().replace('\'', "'\\''")
    );
    let options = call.arguments.get(1).unwrap_or(&Value::Null);
    let payload = ToolPayload::Function { arguments: json!({"cmd":command,"login":false,"yield_time_ms":1000,"timeout_ms":options["timeoutMs"],"max_output_tokens":8192,"workdir":options["cwd"]}).to_string() };
    let cancellation_token = CancellationToken::new();
    let _cancel_on_drop = cancellation_token.clone().drop_guard();
    let call_id = format!("mod-{}", uuid::Uuid::new_v4());
    session
        .services
        .claude_mods
        .register_process_call(call_id.clone());
    let invocation = ToolInvocation {
        session: Arc::clone(session),
        turn: Arc::clone(&step.turn),
        step_context: step,
        cancellation_token,
        tracker: Arc::new(Mutex::new(Default::default())),
        call_id,
        tool_name: ToolName::plain("exec_command"),
        source: ToolCallSource::Direct,
        payload: payload.clone(),
    };
    let handler = ExecCommandHandler::one_shot(ExecCommandHandlerOptions {
        include_login_parameter: false,
        allow_tty: false,
        exec_permission_approvals_enabled: true,
        include_environment_id: false,
        include_shell_parameter: true,
        include_windows_shell_guidance: cfg!(windows),
    });
    let result = handler
        .handle(invocation)
        .await
        .map_err(|error| error.to_string())?;
    let output = result.code_mode_result(&payload);
    Ok(
        json!({"stdout":output["output"],"stderr":"","exitCode":output["exit_code"],"isError":!result.success_for_logging()}),
    )
}
