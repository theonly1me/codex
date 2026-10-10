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

fn context(home: &Path) -> ModSessionContext {
    ModSessionContext {
        codex_home: home.into(),
        cwd: home.display().to_string(),
        session_id: "loader".into(),
        interactive: true,
        delegate: Arc::new(RejectHost),
        sink: Arc::new(|_| Box::pin(async {})),
    }
}

fn fixture(root: &Path) -> Result<InstalledModLoader, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(root.join(".claude-plugin"))?;
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        r#"{"name":"fixture","hooks":{"modules":["./register.ts"]}}"#,
    )?;
    std::fs::write(
        root.join("register.ts"),
        r#"
export function register(on) {
  on('session.start', async ($, event, next) => {
    await $.command.register({name:'answer'});
    return next(event);
  });
  on('command.run', () => ({text:'loaded through a hook'}));
}
"#,
    )?;
    Ok(InstalledModLoader {
        sources: vec![ModSource {
            id: "fixture@test".into(),
            root: root.into(),
            enabled: true,
            active: true,
            options: Map::new(),
        }],
    })
}

struct SnapshotLoader(LoadedMods);

impl ModLoader for SnapshotLoader {
    fn load(&self) -> ModResult<LoadedMods> {
        Ok(LoadedMods {
            enabled: self.0.enabled.clone(),
            plugins: self.0.plugins.clone(),
        })
    }
}

#[tokio::test]
async fn a_replacement_loader_runs_commands_without_a_native_plugin_manager()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let installed = fixture(&home.path().join("plugin"))?;
    let replacement = SnapshotLoader(installed.load()?);
    let session = ModSession::default();
    session
        .load(&replacement, || async { Ok(context(home.path())) })
        .await?;
    assert_eq!(
        session.run_command("fixture@test", "answer", "").await?,
        "loaded through a hook"
    );
    session.stop().await?;
    Ok(())
}

#[tokio::test]
async fn an_inactive_native_plugin_does_not_start_executable_code()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let mut loader = fixture(&home.path().join("plugin"))?;
    for (enabled, active) in [(true, false), (false, true), (false, false)] {
        loader.sources[0].enabled = enabled;
        loader.sources[0].active = active;
        let session = ModSession::default();
        session
            .load(&loader, || async {
                Err("inactive plugin requested a host".into())
            })
            .await?;
        assert!(session.commands().await.is_empty());
        assert!(session.runtime().is_none());
    }
    Ok(())
}

#[tokio::test]
async fn a_disappeared_module_blocks_commands_until_explicit_disablement()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let mut loader = fixture(&home.path().join("plugin"))?;
    let session = ModSession::default();
    session
        .load(&loader, || async { Ok(context(home.path())) })
        .await?;
    loader.sources[0].active = false;
    let failure = session
        .load(&loader, || async { Ok(context(home.path())) })
        .await;
    assert!(
        failure
            .unwrap_err()
            .contains("disable the plugin explicitly")
    );
    assert!(
        session
            .run_command("fixture@test", "answer", "")
            .await
            .is_err()
    );
    loader.sources[0].enabled = false;
    session
        .load(&loader, || async { Ok(context(home.path())) })
        .await?;
    assert!(session.runtime().is_none());
    assert!(session.commands().await.is_empty());
    assert!(session.check().is_ok());
    session.stop().await?;
    Ok(())
}
