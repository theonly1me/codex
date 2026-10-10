use crate::ModPlugin;
use crate::ModResult;
use codex_claude_mod_runtime::ModPluginRequest;
use serde_json::Map;
use serde_json::Value;
use std::path::PathBuf;

pub trait ModLoader: Send + Sync {
    fn load(&self) -> ModResult<LoadedMods>;
}

pub struct LoadedMods {
    pub enabled: Vec<String>,
    pub plugins: Vec<ModPlugin>,
}

pub struct ModSource {
    pub id: String,
    pub root: PathBuf,
    pub enabled: bool,
    pub active: bool,
    pub options: Map<String, Value>,
}

pub struct InstalledModLoader {
    pub sources: Vec<ModSource>,
}

impl ModLoader for InstalledModLoader {
    fn load(&self) -> ModResult<LoadedMods> {
        let mut loaded = LoadedMods {
            enabled: Vec::new(),
            plugins: Vec::new(),
        };
        for source in &self.sources {
            if source.enabled {
                loaded.enabled.push(source.id.clone());
            }
            if !source.enabled || !source.active {
                continue;
            }
            if let Some(plugin) = ModPlugin::discover(ModPluginRequest {
                id: &source.id,
                root: &source.root,
                options: &source.options,
            })
            .map_err(|error| format!("{}: {error}", source.id))?
            {
                loaded.plugins.push(plugin);
            }
        }
        Ok(loaded)
    }
}
