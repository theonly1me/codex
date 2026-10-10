use crate::session::session::Session;
use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::ModResult;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn run(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let first = call.arguments.first().unwrap_or(&Value::Null);

    let command = first
        .as_str()
        .or_else(|| first["command"].as_str())
        .ok_or_else(|| "command.run requires a command".to_string())?;
    let args = first["args"]
        .as_str()
        .or_else(|| call.arguments.get(1).and_then(Value::as_str))
        .unwrap_or_default();
    if command == "theme" && args.is_empty() {
        session
            .send_event_raw(Event {
                id: "claude-mods".into(),
                msg: EventMsg::ClaudeMod(ClaudeModEvent::ThemePicker),
            })
            .await;
        return Ok(json!({"text":""}));
    }
    let candidates = session
        .services
        .claude_mods
        .commands()
        .await
        .into_iter()
        .filter(|entry| entry.name == command || entry.qualified_name == command)
        .collect::<Vec<_>>();
    let registered = candidates
        .iter()
        .find(|entry| entry.plugin_id == call.plugin)
        .or_else(|| {
            if candidates.len() == 1 {
                candidates.first()
            } else {
                None
            }
        })
        .ok_or_else(|| format!("unknown or ambiguous mod command {command}"))?;
    session
        .services
        .claude_mods
        .run_command(&registered.plugin_id, &registered.name, args)
        .await
        .map(|text| json!({"text":text}))
}
