mod support;

use codex_claude_mod_runtime::Dispatch;
use codex_claude_mod_runtime::ModRuntime;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn extensionless_typescript_imports_reach_the_real_event_handler()
-> Result<(), Box<dyn std::error::Error>> {
    let (directory, plugin) = support::plugin(
        "import {answer} from './answer'; export function register(on) { on('command.run', async () => ({text:answer})); }",
    )?;
    std::fs::write(
        directory.path().join("hooks/answer.ts"),
        "export const answer: string = 'loaded local module';",
    )?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let result = runtime
        .observe(Dispatch {
            name: "command.run".into(),
            input: json!({"command":"answer"}),
        })
        .await?;
    assert_eq!(result["text"], "loaded local module");
    Ok(())
}

#[test]
fn missing_entry_module_reports_its_path() -> Result<(), Box<dyn std::error::Error>> {
    let (directory, _plugin) = support::plugin("export function register() {}")?;
    std::fs::remove_file(directory.path().join("hooks/register.tsx"))?;
    let result = support::discover(directory.path());
    let error = result.err().ok_or("missing module was accepted")?;
    assert!(error.contains("register.tsx"), "{error}");
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn symlinked_import_cannot_escape_the_plugin_root() -> Result<(), Box<dyn std::error::Error>>
{
    let (directory, plugin) = support::plugin("import './outside'; export function register() {}")?;
    let outside = tempfile::tempdir()?;
    let target = outside.path().join("outside.ts");
    std::fs::write(&target, "throw Error('must not execute');")?;
    std::os::unix::fs::symlink(target, directory.path().join("hooks/outside.ts"))?;
    let outcome = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await;
    let error = outcome.err().ok_or("escaping import was accepted")?;
    assert!(error.contains("escapes plugin root"), "{error}");
    Ok(())
}
