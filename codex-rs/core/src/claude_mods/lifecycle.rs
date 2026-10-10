use crate::session::TurnInput;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_protocol::error::CodexErr;
use codex_protocol::models::BaseInstructions;
use codex_protocol::user_input::UserInput;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn turn_start(
    session: &Arc<Session>,
    turn: &TurnContext,
    input: &[TurnInput],
) -> Result<(), CodexErr> {
    let text = input
        .iter()
        .filter_map(|item| match item {
            TurnInput::UserInput { content, .. } => Some(content),
            _ => None,
        })
        .flatten()
        .filter_map(|item| match item {
            UserInput::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    session
        .services
        .claude_mods
        .observe(
            "turn.start",
            scoped(turn, json!({"turnId":turn.sub_id,"text":text})),
        )
        .await
        .map_err(CodexErr::InvalidRequest)?;
    Ok(())
}

pub(crate) async fn compose(
    session: &Session,
    mut instructions: BaseInstructions,
) -> Result<BaseInstructions, CodexErr> {
    session
        .services
        .claude_mods
        .check()
        .map_err(CodexErr::InvalidRequest)?;
    if session.services.claude_mods.runtime().is_none() {
        return Ok(instructions);
    }
    let result = session
        .services
        .claude_mods
        .observe(
            "prompt.compose",
            json!({"sections":[{"id":"codex","text":instructions.text,"scope":"shared"}]}),
        )
        .await
        .map_err(CodexErr::InvalidRequest)?;
    let sections = result["sections"].as_array().ok_or_else(|| {
        CodexErr::InvalidRequest("mod prompt.compose must return sections".into())
    })?;
    instructions.text = sections
        .iter()
        .map(|section| {
            section["text"]
                .as_str()
                .ok_or_else(|| CodexErr::InvalidRequest("mod prompt section requires text".into()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n\n");
    Ok(instructions)
}

pub(crate) async fn turn_complete(
    session: &Session,
    turn: &TurnContext,
    answer: Option<&str>,
    is_aborted: bool,
    duration_ms: Option<i64>,
) {
    let event = scoped(
        turn,
        json!({"turnId":turn.sub_id,"answer":answer.unwrap_or_default(),"text":answer.unwrap_or_default(),"reason":if is_aborted {"aborted"} else {"answer"},"isAborted":is_aborted,"durationMs":duration_ms}),
    );
    if let Err(message) = session
        .services
        .claude_mods
        .observe("turn.complete", event)
        .await
    {
        report(session, turn, message).await;
    }
}

pub(crate) fn scoped(turn: &TurnContext, mut event: Value) -> Value {
    if matches!(
        turn.session_source,
        codex_protocol::protocol::SessionSource::SubAgent(_)
    ) {
        event["agentId"] = json!(turn.sub_id);
    }
    event
}

pub(crate) async fn measure(session: &Session, turn: &TurnContext) {
    if session.services.claude_mods.runtime().is_none() {
        return;
    }
    let tokens = session.get_total_token_usage().await;
    let window = turn.model_context_window().unwrap_or(0);
    let percent = if window > 0 {
        tokens as f64 * 100.0 / window as f64
    } else {
        0.0
    };
    let event = scoped(
        turn,
        json!({"changed":["context"],"context":{"tokens":tokens,"window":window,"percent":percent}}),
    );
    if let Err(message) = session
        .services
        .claude_mods
        .observe("session.measure", event)
        .await
    {
        report(session, turn, message).await;
    }
}

pub(crate) async fn compacted(session: &Session, turn: &TurnContext) {
    if let Err(message) = session
        .services
        .claude_mods
        .observe(
            "classic.SessionStart",
            scoped(
                turn,
                json!({"source":"compact","session_id":session.thread_id.to_string()}),
            ),
        )
        .await
    {
        report(session, turn, message).await;
    }
}

async fn report(session: &Session, turn: &TurnContext, message: String) {
    session
        .send_event_raw(codex_protocol::protocol::Event {
            id: turn.sub_id.clone(),
            msg: codex_protocol::protocol::EventMsg::ClaudeMod(
                codex_protocol::claude_mods::ClaudeModEvent::Error {
                    plugin_id: "runtime".into(),
                    message,
                },
            ),
        })
        .await;
}
