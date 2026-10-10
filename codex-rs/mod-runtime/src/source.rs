use crate::ModResult;
use crate::resolve_options;
use serde_json::Map;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ModPlugin {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub module: PathBuf,
    pub additional_modules: Vec<PathBuf>,
    pub source_signature: u64,
    pub options: Value,
    pub remapped_models: Vec<String>,
}

pub struct ModPluginRequest<'a> {
    pub id: &'a str,
    pub root: &'a Path,
    pub options: &'a Map<String, Value>,
}

pub enum ModHookSource {
    File(PathBuf),
    Inline(Value),
}

impl ModPlugin {
    pub fn discover(request: ModPluginRequest<'_>) -> ModResult<Option<Self>> {
        let Some(path) = codex_utils_plugins::find_plugin_manifest_path(request.root) else {
            return Ok(None);
        };
        let manifest: Value =
            serde_json::from_str(&read_source(&path)?).map_err(|error| error.to_string())?;
        let hooks = crate::hook_sources::from_manifest(request.root, manifest.get("hooks"));
        let explicit_name = None;
        Self::discover_resolved(request, explicit_name, hooks)
    }

    pub fn discover_resolved(
        request: ModPluginRequest<'_>,
        name: Option<&str>,
        hooks: Vec<ModHookSource>,
    ) -> ModResult<Option<Self>> {
        let root = request
            .root
            .canonicalize()
            .map_err(|error| format!("{}: {error}", request.root.display()))?;
        let mut modules = crate::hook_sources::modules(&root, hooks)?;
        if modules.is_empty() {
            return Ok(None);
        }
        let path = codex_utils_plugins::find_plugin_manifest_path(&root)
            .ok_or_else(|| "mod manifest disappeared".to_string())?;
        let manifest: Value =
            serde_json::from_str(&read_source(&path)?).map_err(|error| error.to_string())?;
        let definitions = match manifest.get("userConfig") {
            Some(value) => value
                .as_object()
                .ok_or_else(|| "mod userConfig must be an object".to_string())?,
            None => &Map::new(),
        };
        let name = name
            .or_else(|| manifest["name"].as_str())
            .filter(|name| !name.trim().is_empty())
            .or_else(|| root.file_name().and_then(|name| name.to_str()))
            .ok_or_else(|| "mod has no name".to_string())?
            .to_string();
        let mut options = resolve_options(definitions, request.options)?;
        let remapped_models =
            crate::hook_sources::remap_models(definitions, request.options, &mut options);
        let source_signature = crate::source_signature::compute(&root)?;
        let module = modules.remove(0);
        Ok(Some(Self {
            id: request.id.into(),
            name,
            root,
            module,
            additional_modules: modules,
            source_signature,
            options,
            remapped_models,
        }))
    }
}

pub(crate) fn read_source(path: &Path) -> ModResult<String> {
    let metadata = fs::metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if metadata.len() > 2 * 1024 * 1024 || !metadata.is_file() {
        return Err(format!(
            "{}: mod source must be a file smaller than 2 MiB",
            path.display()
        ));
    }
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}
