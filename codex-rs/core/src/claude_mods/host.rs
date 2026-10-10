use crate::session::session::Session;
use crate::session::step_context::StepContext;
use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::ModHost;
use codex_claude_mods_extension::ModResult;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use futures::future::BoxFuture;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::sync::Weak;
use tokio_util::sync::CancellationToken;

pub(crate) struct CoreHost {
    pub session: Weak<Session>,
}

impl ModHost for CoreHost {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move {
            let session = self
                .session
                .upgrade()
                .ok_or_else(|| "mod session stopped".to_string())?;
            handle(&session, &call).await
        })
    }
}

async fn handle(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let first = call.arguments.first().unwrap_or(&Value::Null);
    match call.method.as_str() {
        "model.complete" | "model.fork" => super::model::complete(session, call).await,
        "process.run" => super::process::run(session, call).await,
        "ui.ask" => super::questions::ask(session, call).await,
        "env.get" => Ok(std::env::var(string(first)?)
            .ok()
            .map(Value::String)
            .unwrap_or(Value::Null)),
        "fs.read" | "fs.write" | "fs.exists" | "fs.stat" | "fs.list" | "fs.ancestors" => {
            super::files::call(session, call).await
        }
        "session.id" => Ok(json!(session.thread_id.to_string())),
        "session.cwd" | "session.root" => Ok(json!(
            step(session)
                .await?
                .environments()
                .first()
                .ok_or_else(|| "mod has no environment".to_string())?
                .0
                .cwd()
                .to_abs_path()
                .map_err(|error| error.to_string())?
                .as_path()
                .display()
                .to_string()
        )),
        "session.model" => Ok(json!(step(session).await?.settings.model_info.slug)),
        "session.version" => Ok(json!(env!("CARGO_PKG_VERSION"))),
        "session.messages" => serde_json::to_value(
            session
                .conversation_history_snapshot()
                .await
                .items()
                .collect::<Vec<_>>(),
        )
        .map_err(|error| error.to_string()),
        "session.usage" => {
            let step = step(session).await?;
            let tokens = session.get_total_token_usage().await;
            let window = step.turn.model_context_window().unwrap_or(0);
            Ok(
                json!({"context":{"tokens":tokens,"window":window,"percent":if window > 0 {tokens as f64 * 100.0 / window as f64} else {0.0}},"rateLimits":[],"cost":null}),
            )
        }
        "prompt.submit" => submit(session, call).await,
        "session.append" => append(session, call).await,
        "prompt.fill" => {
            let text = first
                .as_str()
                .or_else(|| first["text"].as_str())
                .ok_or_else(|| "prompt.fill requires text".to_string())?;
            session
                .send_event_raw(Event {
                    id: "claude-mods".into(),
                    msg: EventMsg::ClaudeMod(ClaudeModEvent::PromptFill {
                        plugin_id: call.plugin.clone(),
                        text: text.into(),
                    }),
                })
                .await;
            Ok(Value::Null)
        }
        "command.run" => super::commands::run(session, call).await,
        _ => Err(format!("unsupported mod API {}", call.method)),
    }
}

pub(crate) async fn step(session: &Arc<Session>) -> ModResult<Arc<StepContext>> {
    let active = {
        session
            .active_turn
            .lock()
            .await
            .as_ref()
            .map(|active| Arc::clone(&active.turn_state))
    };
    if let Some(active) = active {
        let captured = active.lock().await.last_known_step_context.clone();
        if let Some(step) = captured {
            return Ok(step);
        }
    }
    let turn = session.new_default_turn().await;
    session
        .capture_step_context(turn, &CancellationToken::new())
        .await
        .map_err(|error| error.to_string())
}

async fn submit(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let options = call.arguments.first().unwrap_or(&Value::Null);
    let text = string(&options["text"])?;
    let text = if options["asUser"].as_bool() == Some(true) {
        text.into()
    } else {
        format!("The {} mod sent this message:\n{text}", call.plugin)
    };
    let item = super::message("user", text.clone());
    if session.inject_if_running(vec![item]).await.is_err() {
        let mut request = crate::TurnInputRequest::user_input(vec![
            codex_protocol::user_input::UserInput::Text {
                text,
                text_elements: Vec::new(),
            },
        ]);
        request.start.turn_trigger = Some(format!("claude-mod:{}", call.plugin));
        session
            .submit_extension_prompt(request)
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(json!({"isSubmitted":true}))
}

pub(crate) fn string(value: &Value) -> ModResult<&str> {
    value
        .as_str()
        .ok_or_else(|| "mod API requires a string".to_string())
}

async fn append(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let message = &call.arguments.first().unwrap_or(&Value::Null)["message"];
    if message["type"] != "user" {
        return Err("mods can append only user messages".into());
    }
    let content = message["content"]
        .as_array()
        .ok_or_else(|| "mod message requires content".to_string())?;
    let text = content
        .iter()
        .map(|part| string(&part["text"]))
        .collect::<ModResult<Vec<_>>>()?
        .join("\n");
    match session
        .inject_if_running(vec![super::message(
            "user",
            format!("Mod {}:\n{text}", call.plugin),
        )])
        .await
    {
        Ok(()) => Ok(json!({"isAppended":true})),
        Err(_) => Ok(json!({"deny":"the turn is no longer running"})),
    }
}
