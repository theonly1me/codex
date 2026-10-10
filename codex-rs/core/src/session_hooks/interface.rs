use crate::WithTurnExtensionData;
use crate::client_common::ResponseStream;
use crate::config::Config;
use crate::function_tool::FunctionCallError;
use crate::session::TurnInput;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::ToolCallState;
use crate::tools::context::ToolInvocation;
use crate::tools::registry::AnyToolResult;
use codex_protocol::error::CodexErr;
use codex_protocol::models::BaseInstructions;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use codex_protocol::turn_input::TurnInputRequest;
use futures::future::BoxFuture;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(crate) type ToolContinuation<'a> = Box<
    dyn FnMut(
            ToolInvocation,
            Option<Arc<ToolCallState>>,
        ) -> BoxFuture<'a, Result<AnyToolResult, FunctionCallError>>
        + Send
        + 'a,
>;

#[derive(Clone, Copy)]
pub(crate) struct TurnCompletion<'a> {
    pub answer: Option<&'a str>,
    pub is_aborted: bool,
    pub duration_ms: Option<i64>,
}

pub(crate) trait SessionHook:
    codex_extension_api::ThreadLifecycleContributor<Config>
    + codex_extension_api::TurnLifecycleContributor
{
    fn start<'a>(
        &'a self,
        session: &'a Arc<Session>,
        config: &'a Config,
        source: &'a SessionSource,
        disabled: &'a [String],
    ) -> BoxFuture<'a, Result<(), String>> {
        let _ = (session, config, source, disabled);
        Box::pin(async { Ok(()) })
    }

    fn submit<'a>(
        &'a self,
        session: &'a Session,
        request: &'a mut WithTurnExtensionData<TurnInputRequest>,
    ) -> BoxFuture<'a, Result<bool, CodexErr>> {
        let _ = (session, request);
        Box::pin(async { Ok(true) })
    }

    fn turn_start<'a>(
        &'a self,
        session: &'a Arc<Session>,
        turn: &'a TurnContext,
        input: &'a [TurnInput],
    ) -> BoxFuture<'a, Result<(), CodexErr>> {
        let _ = (session, turn, input);
        Box::pin(async { Ok(()) })
    }

    fn compose<'a>(
        &'a self,
        session: &'a Session,
        instructions: BaseInstructions,
    ) -> BoxFuture<'a, Result<BaseInstructions, CodexErr>> {
        let _ = session;
        Box::pin(async { Ok(instructions) })
    }

    fn stream(
        &self,
        session: &Session,
        turn: &TurnContext,
        stream: ResponseStream,
        cancellation: CancellationToken,
    ) -> ResponseStream {
        let _ = (session, turn, cancellation);
        stream
    }

    fn complete<'a>(
        &'a self,
        session: &'a Session,
        turn: &'a TurnContext,
        completion: TurnCompletion<'a>,
    ) -> BoxFuture<'a, ()> {
        let _ = (session, turn, completion);
        Box::pin(async {})
    }

    fn measure<'a>(&'a self, session: &'a Session, turn: &'a TurnContext) -> BoxFuture<'a, ()> {
        let _ = (session, turn);
        Box::pin(async {})
    }

    fn accepts_event(&self, session: &Session, event: &EventMsg) -> bool {
        let _ = (session, event);
        true
    }

    fn dispatch<'a>(
        &'a self,
        invocation: ToolInvocation,
        call_state: Option<Arc<ToolCallState>>,
        mut next: ToolContinuation<'a>,
    ) -> BoxFuture<'a, Result<AnyToolResult, FunctionCallError>> {
        next(invocation, call_state)
    }
}
