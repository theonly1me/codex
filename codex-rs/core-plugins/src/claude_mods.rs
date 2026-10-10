use crate::PluginLoadOutcome;
use crate::PluginsConfigInput;
use crate::manifest::PluginManifestHooks;
use crate::marketplace_policy::configured_plugins_from_stack;
use codex_claude_mod_runtime::ModHookSource;
use codex_claude_mod_runtime::ModPlugin;
use codex_claude_mod_runtime::ModPluginRequest;
use std::path::Path;

pub fn load_claude_mod_plugins(
    loaded: &PluginLoadOutcome,
    config: &PluginsConfigInput,
    codex_home: &Path,
) -> Result<Vec<ModPlugin>, String> {
    let configurations = configured_plugins_from_stack(&config.config_layer_stack, codex_home);
    let empty_options = serde_json::Map::new();
    let mut modules = Vec::new();
    for plugin in loaded.plugins().iter().filter(|plugin| plugin.is_active()) {
        let options = configurations
            .get(&plugin.config_name)
            .map(|configuration| &configuration.options)
            .unwrap_or(&empty_options);
        let Some(manifest) = crate::manifest::load_plugin_manifest(plugin.root.as_path()) else {
            continue;
        };
        if manifest.paths.hooks.is_none() {
            ModPlugin::discover(ModPluginRequest {
                id: &plugin.config_name,
                root: plugin.root.as_path(),
                options,
            })?;
        }
        let sources = match manifest.paths.hooks {
            Some(PluginManifestHooks::Paths(paths)) => paths
                .into_iter()
                .map(|path| ModHookSource::File(path.into_path_buf()))
                .collect(),
            Some(PluginManifestHooks::Inline(files)) => files
                .into_iter()
                .map(|file| serde_json::to_value(file).map(ModHookSource::Inline))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| error.to_string())?,
            None => vec![ModHookSource::File(
                plugin.root.join("hooks/hooks.json").into_path_buf(),
            )],
        };
        if let Some(module) = ModPlugin::discover_resolved(
            ModPluginRequest {
                id: &plugin.config_name,
                root: plugin.root.as_path(),
                options,
            },
            Some(&manifest.name),
            sources,
        )
        .map_err(|error| format!("{}: {error}", plugin.config_name))?
        {
            modules.push(module);
        }
    }
    Ok(modules)
}
