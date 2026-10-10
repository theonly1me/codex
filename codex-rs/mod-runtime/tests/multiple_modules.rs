mod support;

use codex_mod_runtime::Dispatch;
use codex_mod_runtime::ModRuntime;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn every_declared_module_registers_its_middleware() -> Result<(), Box<dyn std::error::Error>>
{
    let (directory, _) = support::plugin(
        "export function register(on) { on('command.run', async ($, event, next) => ({...await next(event), first:true})); }",
    )?;
    std::fs::write(
        directory.path().join("hooks/second.ts"),
        "export function register(on) { on('command.run', () => ({text:'second module'})); }",
    )?;
    std::fs::write(
        directory.path().join("hooks/hooks.json"),
        r#"{"modules":["./register.tsx","./second.ts"]}"#,
    )?;
    let plugin = support::discover(directory.path())?.ok_or("missing modules")?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let result = runtime
        .observe(Dispatch {
            name: "command.run".into(),
            input: json!({"command":"fixture"}),
        })
        .await?;
    assert_eq!(result, json!({"text":"second module","first":true}));
    Ok(())
}
