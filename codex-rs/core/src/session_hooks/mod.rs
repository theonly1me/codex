mod dispatch;
mod interface;
mod lifecycle;

use crate::config::Config;
use crate::session::session::Session;
use codex_extension_api::ExtensionRegistry;
use std::sync::Arc;

pub(crate) use dispatch::dispatch;
pub(crate) use interface::SessionHook;
pub(crate) use interface::ToolContinuation;
pub(crate) use interface::TurnCompletion;

pub(crate) struct SessionHooks {
    handlers: Vec<Arc<dyn SessionHook>>,
}

impl Default for SessionHooks {
    fn default() -> Self {
        Self {
            handlers: vec![Arc::new(crate::claude_mods::ModHooks)],
        }
    }
}

pub(crate) fn for_session(session: &Session) -> Arc<SessionHooks> {
    session
        .services
        .thread_extension_data
        .get_or_init(SessionHooks::default)
}

pub(crate) fn install(
    registry: &Arc<ExtensionRegistry<Config>>,
    store: &codex_extension_api::ExtensionData,
) -> Arc<ExtensionRegistry<Config>> {
    let mut builder = registry.to_builder();
    let hooks = store.get_or_init(SessionHooks::default);
    for handler in &hooks.handlers {
        let contributor: Arc<dyn codex_extension_api::ThreadLifecycleContributor<Config>> =
            handler.clone();
        builder.thread_lifecycle_contributor(contributor);
        let contributor: Arc<dyn codex_extension_api::TurnLifecycleContributor> = handler.clone();
        builder.turn_lifecycle_contributor(contributor);
    }
    Arc::new(builder.build())
}

struct UserInputGate(Arc<tokio::sync::Semaphore>);

pub(crate) async fn acquire_user_input(
    session: &Session,
) -> Option<tokio::sync::OwnedSemaphorePermit> {
    let gate = session
        .services
        .thread_extension_data
        .get_or_init(|| UserInputGate(Arc::new(tokio::sync::Semaphore::new(1))));
    Arc::clone(&gate.0).acquire_owned().await.ok()
}

#[cfg(test)]
mod tests;
