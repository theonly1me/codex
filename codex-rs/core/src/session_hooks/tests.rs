use super::SessionHook;
use super::SessionHooks;
use crate::client_common::ResponseEvent;
use crate::client_common::ResponseStream;
use crate::config::Config;
use crate::session::session::Session;
use codex_extension_api::ThreadLifecycleContributor;
use codex_protocol::error::CodexErr;
use codex_protocol::models::BaseInstructions;
use futures::StreamExt;
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

struct PromptMarker;

impl ThreadLifecycleContributor<Config> for PromptMarker {}
impl codex_extension_api::TurnLifecycleContributor for PromptMarker {}

impl SessionHook for PromptMarker {
    fn compose<'a>(
        &'a self,
        _session: &'a Session,
        mut instructions: BaseInstructions,
    ) -> BoxFuture<'a, Result<BaseInstructions, CodexErr>> {
        Box::pin(async move {
            instructions.text = format!("replacement: {}", instructions.text);
            Ok(instructions)
        })
    }
}

#[tokio::test]
async fn a_host_can_replace_mod_hooks_through_extension_state() {
    let (session, _) = crate::session::tests::make_session_and_context().await;
    session.services.thread_extension_data.insert(SessionHooks {
        handlers: vec![Arc::new(PromptMarker)],
    });
    let instructions = super::for_session(&session)
        .compose(
            &session,
            BaseInstructions {
                text: "native instructions".into(),
                provenance: Some(codex_protocol::models::BaseInstructionsProvenance::Custom),
            },
        )
        .await
        .expect("compose through the replacement hook");
    assert_eq!(instructions.text, "replacement: native instructions");
    assert_eq!(
        instructions.provenance,
        Some(codex_protocol::models::BaseInstructionsProvenance::Custom)
    );
    assert!(
        session
            .services
            .thread_extension_data
            .get::<codex_claude_mods_extension::ModSession>()
            .is_none()
    );
}

#[tokio::test]
async fn an_unplugged_registry_preserves_native_instructions_and_stream_events() {
    let (session, turn) = crate::session::tests::make_session_and_context().await;
    session.services.thread_extension_data.insert(SessionHooks {
        handlers: Vec::new(),
    });
    let hooks = super::for_session(&session);
    let instructions = hooks
        .compose(
            &session,
            BaseInstructions {
                text: "native instructions".into(),
                provenance: Some(codex_protocol::models::BaseInstructionsProvenance::Custom),
            },
        )
        .await
        .expect("native instructions without extensions");
    assert_eq!(instructions.text, "native instructions");
    assert_eq!(
        instructions.provenance,
        Some(codex_protocol::models::BaseInstructionsProvenance::Custom)
    );
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    sender
        .send(Ok(ResponseEvent::OutputTextDelta("native output".into())))
        .await
        .expect("send native output");
    drop(sender);
    let cancellation = CancellationToken::new();
    let stream = ResponseStream {
        rx_event: receiver,
        interrupt: None,
        consumer_dropped: cancellation.clone(),
    };
    let mut output = hooks.stream(&session, &turn, stream, cancellation);
    assert!(
        matches!(output.next().await, Some(Ok(ResponseEvent::OutputTextDelta(text))) if text == "native output")
    );
    assert!(output.next().await.is_none());
    assert!(
        session
            .services
            .thread_extension_data
            .get::<codex_claude_mods_extension::ModSession>()
            .is_none()
    );
}
