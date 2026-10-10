use crate::EventSink;
use crate::ModHost;
use crate::ModLoader;
use crate::ModResult;
use crate::ModSession;
use crate::ModSessionRequest;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ModSessionContext {
    pub codex_home: PathBuf,
    pub cwd: String,
    pub session_id: String,
    pub interactive: bool,
    pub delegate: Arc<dyn ModHost>,
    pub sink: EventSink,
}

impl ModSession {
    pub async fn load<ContextFactory, ContextFuture>(
        &self,
        loader: &dyn ModLoader,
        context: ContextFactory,
    ) -> ModResult<()>
    where
        ContextFactory: FnOnce() -> ContextFuture,
        ContextFuture: Future<Output = ModResult<ModSessionContext>>,
    {
        let loaded = match loader.load() {
            Ok(loaded) => loaded,
            Err(error) => {
                self.discovery_failed(error.clone());
                return Err(error);
            }
        };
        self.validate_reload(&loaded.enabled, &loaded.plugins)?;
        if loaded.plugins.is_empty() && self.runtime().is_none() && self.check().is_ok() {
            return Ok(());
        }
        let context = context().await?;
        self.start(ModSessionRequest {
            plugins: loaded.plugins,
            codex_home: context.codex_home,
            cwd: context.cwd,
            session_id: context.session_id,
            interactive: context.interactive,
            delegate: context.delegate,
            sink: context.sink,
        })
        .await
    }
}
