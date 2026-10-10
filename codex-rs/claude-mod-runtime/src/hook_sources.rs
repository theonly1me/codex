use crate::ModResult;
use crate::source::ModHookSource;
use crate::source::read_source;
use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use std::path::PathBuf;

pub(crate) fn from_manifest(root: &Path, hooks: Option<&Value>) -> Vec<ModHookSource> {
    match hooks {
        Some(Value::String(path)) => vec![ModHookSource::File(root.join(path))],
        Some(Value::Array(entries)) => entries
            .iter()
            .flat_map(|entry| from_manifest(root, Some(entry)))
            .collect(),
        Some(value @ Value::Object(_)) => vec![ModHookSource::Inline(value.clone())],
        _ => vec![ModHookSource::File(root.join("hooks/hooks.json"))],
    }
}

pub(crate) fn modules(root: &Path, hooks: Vec<ModHookSource>) -> ModResult<Vec<PathBuf>> {
    let mut modules = Vec::new();
    for source in hooks {
        let (parent, document) = match source {
            ModHookSource::Inline(value) => (root.to_path_buf(), value),
            ModHookSource::File(path) => {
                let text = match read_source(&path) {
                    Ok(text) => text,
                    Err(_) => continue,
                };
                let document = match serde_json::from_str::<Value>(&text) {
                    Ok(document) => document,
                    Err(error) if text.contains("\"modules\"") => {
                        return Err(format!("{}: {error}", path.display()));
                    }
                    Err(_) => continue,
                };
                let parent = path
                    .parent()
                    .ok_or_else(|| "hook file has no parent".to_string())?
                    .to_path_buf();
                (parent, document)
            }
        };
        let Some(entries) = document.get("modules") else {
            continue;
        };
        let entries = entries
            .as_array()
            .ok_or_else(|| "mod modules must be an array".to_string())?;
        for entry in entries {
            let relative = entry
                .as_str()
                .filter(|value| value.starts_with("./"))
                .ok_or_else(|| "mod module paths must start with ./".to_string())?;
            let path = parent
                .join(relative)
                .canonicalize()
                .map_err(|error| format!("{}: {error}", parent.join(relative).display()))?;
            if !path.starts_with(root) {
                return Err(format!(
                    "{}: module escapes plugin root {}",
                    path.display(),
                    root.display()
                ));
            }
            if !modules.contains(&path) {
                modules.push(path);
            }
        }
    }
    Ok(modules)
}

pub(crate) fn remap_models(
    definitions: &Map<String, Value>,
    overrides: &Map<String, Value>,
    options: &mut Value,
) -> Vec<String> {
    let mut remapped = Vec::new();
    for (name, definition) in definitions {
        if (name == "model" || name.ends_with("Model"))
            && !overrides.contains_key(name)
            && definition["default"].as_str().is_some_and(|model| {
                model.starts_with("claude-") || ["haiku", "sonnet", "opus"].contains(&model)
            })
        {
            options[name] = Value::String("codex:active".into());
            remapped.push(name.clone());
        }
    }
    remapped
}
