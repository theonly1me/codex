use codex_claude_mods_extension::*;
use codex_mod_runtime::ModPluginRequest;
use futures::future::BoxFuture;
use serde_json::Map;
use serde_json::Value;
use std::sync::Arc;

struct RejectHost;

impl ModHost for RejectHost {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move { Err(format!("unexpected host API {}", call.method)) })
    }
}

fn request(plugin: ModPlugin, home: &std::path::Path) -> ModSessionRequest {
    ModSessionRequest {
        plugins: vec![plugin],
        codex_home: home.into(),
        cwd: home.display().to_string(),
        session_id: "session".into(),
        interactive: false,
        delegate: Arc::new(RejectHost),
        sink: Arc::new(|_| Box::pin(async {})),
    }
}

fn plugin(root: &std::path::Path) -> ModResult<ModPlugin> {
    codex_mod_runtime::ModPlugin::discover(ModPluginRequest {
        id: "fixture@test",
        root,
        options: &Map::new(),
    })?
    .ok_or_else(|| "no executable module".into())
}

#[tokio::test]
async fn ui_attachment_enables_commands_and_reload_reads_changed_source()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("plugin");
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
    if (event.isInteractive) await $.command.register({name:'answer'});
    await $.fs.write((await $.env.get('HOME')) + '/.claude/owned.txt', 'Codex data');
    return next(event);
  });
  on('command.run', () => ({text:'before reload'}));
}
"#;
    std::fs::write(&module, source)?;
    let session = ModSession::default();
    session
        .start(request(plugin(&root)?, directory.path()))
        .await?;
    assert!(session.commands().await.is_empty());
    let interactive = true;
    session.attach_ui(interactive).await?;
    assert_eq!(session.commands().await.len(), 1);
    assert_eq!(
        session.run_command("fixture@test", "answer", "").await?,
        "before reload"
    );
    assert_eq!(
        std::fs::read_to_string(directory.path().join("claude-mods/home/.claude/owned.txt"))?,
        "Codex data"
    );
    std::fs::write(&module, source.replace("before reload", "after reload"))?;
    session
        .start(request(plugin(&root)?, directory.path()))
        .await?;
    assert_eq!(
        session.run_command("fixture@test", "answer", "").await?,
        "after reload"
    );
    assert!(
        session
            .validate_reload(&["fixture@test".into()], &[])
            .is_err()
    );
    assert!(
        session
            .observe("tool.call", serde_json::json!({}))
            .await
            .is_err()
    );
    let disabled = ModSessionRequest {
        plugins: Vec::new(),
        ..request(plugin(&root)?, directory.path())
    };
    session.start(disabled).await?;
    assert!(session.runtime().is_none());
    assert!(session.commands().await.is_empty());
    session.stop().await?;
    Ok(())
}
