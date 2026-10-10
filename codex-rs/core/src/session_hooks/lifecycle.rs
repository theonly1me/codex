use super::SessionHooks;
use super::TurnCompletion;
use crate::WithTurnExtensionData;
use crate::client_common::ResponseStream;
use crate::config::Config;
use crate::session::TurnInput;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_protocol::error::CodexErr;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::turn_input::TurnInputRequest;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

impl SessionHooks {
    pub(crate) async fn start(
        &self,
        session: &Arc<Session>,
        config: &Config,
        source: &SessionSource,
        disabled: &[String],
    ) -> Result<(), String> {
        for handler in &self.handlers {
            handler.start(session, config, source, disabled).await?;
        }
        Ok(())
    }

    pub(crate) async fn submit(
        &self,
        session: &Arc<Session>,
        request: &mut WithTurnExtensionData<TurnInputRequest>,
    ) -> Result<bool, CodexErr> {
        let turn = session.new_default_turn().await;
        self.start(
            session,
            &turn.config,
            &turn.session_source,
            &turn.disabled_plugin_ids,
        )
        .await
        .map_err(CodexErr::InvalidRequest)?;
        for handler in &self.handlers {
            if !handler.submit(session, request).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(crate) async fn turn_start(
        &self,
        session: &Arc<Session>,
        turn: &TurnContext,
        input: &[TurnInput],
    ) -> Result<(), CodexErr> {
        for handler in &self.handlers {
            handler.turn_start(session, turn, input).await?;
        }
        Ok(())
    }

    pub(crate) async fn compose(
        &self,
        session: &Session,
        mut instructions: BaseInstructions,
    ) -> Result<BaseInstructions, CodexErr> {
        for handler in &self.handlers {
            instructions = handler.compose(session, instructions).await?;
        }
        Ok(instructions)
    }

    pub(crate) fn stream(
        &self,
        session: &Session,
        turn: &TurnContext,
        mut stream: ResponseStream,
        cancellation: CancellationToken,
    ) -> ResponseStream {
        for handler in &self.handlers {
            stream = handler.stream(session, turn, stream, cancellation.clone());
        }
        stream
    }

    pub(crate) async fn complete(
        &self,
        session: &Session,
        turn: &TurnContext,
        completion: TurnCompletion<'_>,
    ) {
        for handler in &self.handlers {
            handler.complete(session, turn, completion).await;
        }
    }

    pub(crate) async fn compacted(&self, session: &Session, turn: &TurnContext) {
        for handler in &self.handlers {
            handler.compacted(session, turn).await;
        }
    }

    pub(crate) async fn measure(&self, session: &Session, turn: &TurnContext) {
        for handler in &self.handlers {
            handler.measure(session, turn).await;
        }
    }

    pub(crate) fn accepts_event(&self, session: &Session, event: &EventMsg) -> bool {
        self.handlers
            .iter()
            .all(|handler| handler.accepts_event(session, event))
    }
}
