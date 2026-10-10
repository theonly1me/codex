use crate::WithTurnExtensionData;
use crate::client_common::ResponseStream;
use crate::config::Config;
use crate::function_tool::FunctionCallError;
use crate::session::TurnInput;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::session_hooks::SessionHook;
use crate::session_hooks::ToolContinuation;
use crate::session_hooks::TurnCompletion;
use crate::tools::context::ToolCallState;
use crate::tools::context::ToolInvocation;
use crate::tools::registry::AnyToolResult;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ThreadLifecycleContributor;
use codex_extension_api::ThreadStopInput;
use codex_protocol::error::CodexErr;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::turn_input::TurnInputRequest;
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(crate) struct ModHooks;

impl SessionHook for ModHooks {
    fn start<'a>(
        &'a self,
        session: &'a Arc<Session>,
        config: &'a Config,
        source: &'a SessionSource,
        disabled: &'a [String],
    ) -> BoxFuture<'a, Result<(), String>> {
        Box::pin(super::startup::start(session, config, source, disabled))
    }

    fn submit<'a>(
        &'a self,
        session: &'a Session,
        request: &'a mut WithTurnExtensionData<TurnInputRequest>,
    ) -> BoxFuture<'a, Result<bool, CodexErr>> {
        Box::pin(super::prompt::submit(session, request))
    }

    fn turn_start<'a>(
        &'a self,
        session: &'a Arc<Session>,
        turn: &'a TurnContext,
        input: &'a [TurnInput],
    ) -> BoxFuture<'a, Result<(), CodexErr>> {
        Box::pin(super::lifecycle::turn_start(session, turn, input))
    }

    fn compose<'a>(
        &'a self,
        session: &'a Session,
        instructions: BaseInstructions,
    ) -> BoxFuture<'a, Result<BaseInstructions, CodexErr>> {
        Box::pin(super::lifecycle::compose(session, instructions))
    }

    fn stream(
        &self,
        session: &Session,
        turn: &TurnContext,
        stream: ResponseStream,
        cancellation: CancellationToken,
    ) -> ResponseStream {
        super::stream::wrap(session, turn, stream, cancellation)
    }

    fn complete<'a>(
        &'a self,
        session: &'a Session,
        turn: &'a TurnContext,
        completion: TurnCompletion<'a>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(super::lifecycle::turn_complete(
            session,
            turn,
            completion.answer,
            completion.is_aborted,
            completion.duration_ms,
        ))
    }

    fn compacted<'a>(&'a self, session: &'a Session, turn: &'a TurnContext) -> BoxFuture<'a, ()> {
        Box::pin(super::lifecycle::compacted(session, turn))
    }

    fn measure<'a>(&'a self, session: &'a Session, turn: &'a TurnContext) -> BoxFuture<'a, ()> {
        Box::pin(super::lifecycle::measure(session, turn))
    }

    fn accepts_event(&self, session: &Session, event: &EventMsg) -> bool {
        let call = match event {
            EventMsg::ItemStarted(event) => Some(event.item.id()),
            EventMsg::ItemCompleted(event) => Some(event.item.id()),
            EventMsg::ExecCommandBegin(event) => Some(event.call_id.clone()),
            EventMsg::ExecCommandEnd(event) => Some(event.call_id.clone()),
            EventMsg::ExecCommandOutputDelta(event) => Some(event.call_id.clone()),
            EventMsg::TerminalInteraction(event) => Some(event.call_id.clone()),
            _ => None,
        };
        !call.is_some_and(|id| super::state(session).owns_process_call(&id))
    }

    fn dispatch<'a>(
        &'a self,
        invocation: ToolInvocation,
        call_state: Option<Arc<ToolCallState>>,
        next: ToolContinuation<'a>,
    ) -> BoxFuture<'a, Result<AnyToolResult, FunctionCallError>> {
        Box::pin(super::tools::dispatch(invocation, call_state, next))
    }
}

impl ThreadLifecycleContributor<Config> for ModHooks {
    fn on_thread_stop<'a>(&'a self, input: ThreadStopInput<'a>) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            if let Some(session) = input
                .thread_store
                .get::<codex_claude_mods_extension::ModSession>()
                && let Err(error) = session.stop().await
            {
                tracing::warn!(%error, "Claude mod shutdown failed");
            }
        })
    }
}
