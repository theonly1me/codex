use super::SessionHook;
use crate::function_tool::FunctionCallError;
use crate::tools::context::ToolCallState;
use crate::tools::context::ToolInvocation;
use crate::tools::registry::AnyToolResult;
use crate::tools::registry::ToolRegistry;
use futures::future::BoxFuture;
use std::sync::Arc;

pub(crate) async fn dispatch(
    registry: &ToolRegistry,
    invocation: ToolInvocation,
    call_state: Option<Arc<ToolCallState>>,
) -> Result<AnyToolResult, FunctionCallError> {
    let hooks = super::for_session(&invocation.session);
    chain(&hooks.handlers, registry, invocation, call_state).await
}

fn chain<'a>(
    handlers: &'a [Arc<dyn SessionHook>],
    registry: &'a ToolRegistry,
    invocation: ToolInvocation,
    call_state: Option<Arc<ToolCallState>>,
) -> BoxFuture<'a, Result<AnyToolResult, FunctionCallError>> {
    Box::pin(async move {
        match handlers.split_first() {
            Some((handler, remaining)) => {
                handler
                    .dispatch(
                        invocation,
                        call_state,
                        Box::new(move |invocation, state| {
                            chain(remaining, registry, invocation, state)
                        }),
                    )
                    .await
            }
            None => {
                registry
                    .dispatch_native_with_state(invocation, call_state)
                    .await
            }
        }
    })
}
