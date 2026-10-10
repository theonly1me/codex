use crate::session::session::Session;
use codex_extension_api::ExtensionData;
use codex_extension_api::TurnStartInput;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::items::TurnItem;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use serde_json::json;
use std::sync::Arc;
use std::sync::Weak;

struct Host {
    session: Weak<Session>,
    subagent: bool,
}

struct TurnId(String);

pub(crate) fn bind(session: &Arc<Session>, source: &SessionSource) {
    session.services.thread_extension_data.insert(Host {
        session: Arc::downgrade(session),
        subagent: matches!(source, SessionSource::SubAgent(_)),
    });
}

pub(crate) fn started(input: TurnStartInput<'_>) {
    input.turn_store.insert(TurnId(input.turn_id.to_owned()));
}

pub(crate) async fn completed(
    thread_store: &ExtensionData,
    turn_store: &ExtensionData,
    item: &TurnItem,
) {
    if !matches!(item, TurnItem::ContextCompaction(_)) {
        return;
    }
    let Some(host) = thread_store.get::<Host>() else {
        return;
    };
    let Some(session) = host.session.upgrade() else {
        return;
    };
    let turn = turn_store.get::<TurnId>();
    let mut event = json!({"source":"compact","session_id":session.thread_id.to_string()});
    if host.subagent
        && let Some(turn) = turn.as_ref()
    {
        event["agentId"] = json!(turn.0);
    }
    if let Err(message) = super::state(&session)
        .observe("classic.SessionStart", event)
        .await
    {
        session
            .send_event_raw(Event {
                id: turn.map(|turn| turn.0.clone()).unwrap_or_default(),
                msg: EventMsg::ClaudeMod(ClaudeModEvent::Error {
                    plugin_id: "runtime".into(),
                    message,
                }),
            })
            .await;
    }
}
