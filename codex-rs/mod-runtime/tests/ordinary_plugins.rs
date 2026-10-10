use codex_mod_runtime::ModPlugin;
use codex_mod_runtime::ModPluginRequest;
use serde_json::Map;

fn discover(root: &std::path::Path) -> Result<Option<ModPlugin>, String> {
    ModPlugin::discover(ModPluginRequest {
        id: "fixture@test",
        root,
        options: &Map::new(),
    })
}

#[test]
fn ordinary_plugins_keep_native_manifest_and_hook_compatibility()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    std::fs::create_dir(root.join(".cursor-plugin"))?;
    std::fs::create_dir(root.join("hooks"))?;
    std::fs::write(root.join(".cursor-plugin/plugin.json"), "{}")?;
    std::fs::write(root.join("plugin.json"), "unrelated project data")?;
    std::fs::write(root.join("hooks/hooks.json"), "invalid command hooks")?;
    assert!(discover(root)?.is_none());
    std::fs::write(
        root.join(".cursor-plugin/plugin.json"),
        r#"{"hooks":42,"userConfig":false}"#,
    )?;
    assert!(discover(root)?.is_none());
    Ok(())
}

#[test]
fn executable_modules_still_reject_invalid_declarations() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = tempfile::tempdir()?;
    std::fs::create_dir(directory.path().join(".claude-plugin"))?;
    std::fs::create_dir(directory.path().join("hooks"))?;
    std::fs::write(directory.path().join(".claude-plugin/plugin.json"), "{}")?;
    std::fs::write(
        directory.path().join("hooks/hooks.json"),
        r#"{"modules":42}"#,
    )?;
    assert!(discover(directory.path()).is_err());
    Ok(())
}
