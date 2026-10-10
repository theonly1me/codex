use super::SessionHooks;
use crate::config::Config;
use codex_claude_mods_extension::ModLoader;
use codex_core_plugins::PluginsManager;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionRegistry;
use codex_extension_api::SessionIsolation;
use codex_protocol::protocol::SessionSource;
use std::sync::Arc;

pub(crate) struct HookContext<'a> {
    pub registry: &'a Arc<ExtensionRegistry<Config>>,
    pub store: &'a ExtensionData,
    pub plugins: &'a PluginsManager,
    pub config: &'a Config,
    pub source: &'a SessionSource,
    pub isolation: &'a SessionIsolation,
    pub disabled: &'a [String],
}

pub(crate) async fn prepare(
    input: HookContext<'_>,
) -> Result<Arc<ExtensionRegistry<Config>>, String> {
    if input.store.get::<SessionHooks>().is_none() {
        let mut hooks = SessionHooks {
            handlers: Vec::new(),
        };
        if !matches!(input.source, SessionSource::Internal(_))
            && !matches!(input.isolation, SessionIsolation::Isolated)
        {
            let loader =
                crate::claude_mods::loader::installed(input.plugins, input.config, input.disabled)
                    .await;
            if !loader.load()?.plugins.is_empty() {
                hooks = SessionHooks::default();
            }
        }
        input.store.insert(hooks);
    }
    Ok(super::install(input.registry, input.store))
}
