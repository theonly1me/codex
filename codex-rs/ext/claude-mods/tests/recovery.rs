use codex_claude_mod_runtime::ModPluginRequest;
use codex_claude_mods_extension::*;
use futures::future::BoxFuture;
use serde_json::Map;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

struct RejectHost;

impl ModHost for RejectHost {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move { Err(format!("unexpected host API {}", call.method)) })
    }
}

fn plugin(root: &Path) -> ModResult<ModPlugin> {
    ModPlugin::discover(ModPluginRequest {
        id: "fixture@test",
        root,
        options: &Map::new(),
    })?
    .ok_or_else(|| "fixture has no mod".into())
}

fn request(plugins: Vec<ModPlugin>, home: &Path) -> ModSessionRequest {
    ModSessionRequest {
        plugins,
        codex_home: home.into(),
        cwd: home.display().to_string(),
        session_id: "recovery".into(),
        interactive: false,
        delegate: Arc::new(RejectHost),
        sink: Arc::new(|_| Box::pin(async {})),
    }
}

#[tokio::test]
async fn a_terminated_mod_can_reload_or_be_explicitly_disabled()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let root = home.path().join("plugin");
    std::fs::create_dir_all(root.join(".claude-plugin"))?;
    std::fs::create_dir(root.join("hooks"))?;
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        r#"{"name":"fixture"}"#,
    )?;
    std::fs::write(
        root.join("hooks/hooks.json"),
        r#"{"modules":["./register.ts"]}"#,
    )?;
    let module = root.join("hooks/register.ts");
    let source = r#"
export function register(on) {
  on('session.start', async ($, event, next) => {
    await $.command.register({name:'answer'});
    return next(event);
  });
  on('command.run', ($, event) => {
    if (event.args === 'stop') { while (true) {} }
    return {text:'before reload'};
  });
}
"#;
    std::fs::write(&module, source)?;
    let session = ModSession::default();
    session
        .start(request(vec![plugin(&root)?], home.path()))
        .await?;
    assert!(
        session
            .run_command("fixture@test", "answer", "stop")
            .await
            .is_err()
    );
    session
        .start(request(vec![plugin(&root)?], home.path()))
        .await?;
    assert_eq!(
        session.run_command("fixture@test", "answer", "").await?,
        "before reload"
    );
    std::fs::write(&module, source.replace("before reload", "recovered"))?;
    session
        .start(request(vec![plugin(&root)?], home.path()))
        .await?;
    assert_eq!(
        session.run_command("fixture@test", "answer", "").await?,
        "recovered"
    );
    assert!(
        session
            .run_command("fixture@test", "answer", "stop")
            .await
            .is_err()
    );
    session.start(request(Vec::new(), home.path())).await?;
    assert!(session.runtime().is_none());
    assert!(session.check().is_ok());
    session.stop().await?;
    Ok(())
}
