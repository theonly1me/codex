use super::new_submission_id;
use super::session::Session;
use super::turn_input;
use codex_protocol::error::Result as CodexResult;
use codex_protocol::turn_input::TurnInputMode;
use codex_protocol::turn_input::TurnInputRequest;
use codex_protocol::turn_input::TurnInputSubmission;
use std::sync::Arc;

impl Session {
    pub(crate) async fn submit_extension_prompt(
        self: &Arc<Self>,
        request: TurnInputRequest,
    ) -> CodexResult<TurnInputSubmission> {
        turn_input::handle(
            self,
            request,
            TurnInputMode::StartOrSteer,
            new_submission_id(),
        )
        .await
    }
}
