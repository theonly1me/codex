use crate::config::Config;
use codex_claude_mods_extension::InstalledModLoader;
use codex_claude_mods_extension::ModSource;
use codex_core_plugins::PluginsManager;

pub(crate) async fn installed(
    manager: &PluginsManager,
    config: &Config,
    disabled: &[String],
) -> InstalledModLoader {
    let input = config.plugins_config_input();
    let loaded = manager
        .plugins_for_config(&input)
        .await
        .without_plugins(disabled);
    InstalledModLoader {
        sources: loaded
            .plugins()
            .iter()
            .map(|plugin| ModSource {
                id: plugin.config_name.clone(),
                root: plugin.root.to_path_buf(),
                enabled: plugin.enabled,
                active: plugin.is_active(),
                options: input
                    .plugins
                    .plugins
                    .get(&plugin.config_name)
                    .map(|configuration| configuration.options.clone())
                    .unwrap_or_default(),
            })
            .collect(),
    }
}
