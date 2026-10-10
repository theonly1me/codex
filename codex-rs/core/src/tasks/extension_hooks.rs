use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::session_hooks::TurnCompletion;
use codex_protocol::protocol::TurnAbortReason;

impl Session {
    pub(crate) async fn emit_extension_turn_stop(
        &self,
        turn: &TurnContext,
        answer: Option<&str>,
        duration_ms: Option<i64>,
    ) {
        self.emit_turn_stop_lifecycle(turn.extension_data.as_ref())
            .await;
        crate::session_hooks::for_session(self)
            .complete(
                self,
                turn,
                TurnCompletion {
                    answer,
                    is_aborted: false,
                    duration_ms,
                },
            )
            .await;
    }

    pub(crate) async fn emit_extension_turn_abort(
        &self,
        turn: &TurnContext,
        reason: TurnAbortReason,
        answer: Option<&str>,
        duration_ms: Option<i64>,
    ) {
        self.emit_turn_abort_lifecycle(reason, turn.extension_data.as_ref())
            .await;
        crate::session_hooks::for_session(self)
            .complete(
                self,
                turn,
                TurnCompletion {
                    answer,
                    is_aborted: true,
                    duration_ms,
                },
            )
            .await;
    }
}
