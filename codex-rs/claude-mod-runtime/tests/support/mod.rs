use codex_claude_mod_runtime::HostCall;
use codex_claude_mod_runtime::ModHost;
use codex_claude_mod_runtime::ModPlugin;
use codex_claude_mod_runtime::ModPluginRequest;
use codex_claude_mod_runtime::ModResult;
use futures::future::BoxFuture;
use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use tempfile::TempDir;

pub fn plugin(source: &str) -> Result<(TempDir, ModPlugin), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir(root.join(".claude-plugin"))?;
    std::fs::create_dir(root.join("hooks"))?;
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        r#"{"name":"fixture"}"#,
    )?;
    std::fs::write(
        root.join("hooks/hooks.json"),
        r#"{"modules":["./register.tsx"]}"#,
    )?;
    std::fs::write(root.join("hooks/register.tsx"), source)?;
    let plugin = discover(root)?.ok_or("fixture has no mod")?;
    Ok((directory, plugin))
}

pub fn discover(root: &Path) -> ModResult<Option<ModPlugin>> {
    ModPlugin::discover(ModPluginRequest {
        id: "fixture@test",
        root,
        options: &Map::new(),
    })
}

pub struct RejectHost;

impl ModHost for RejectHost {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move { Err(format!("unexpected host API {}", call.method)) })
    }
}
