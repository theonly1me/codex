use codex_claude_mods_extension::*;
use codex_mod_runtime::ModPluginRequest;
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

fn plugin(root: &Path, id: &str, source: &str) -> ModResult<ModPlugin> {
    std::fs::create_dir_all(root.join(".claude-plugin")).map_err(|error| error.to_string())?;
    std::fs::create_dir(root.join("hooks")).map_err(|error| error.to_string())?;
    std::fs::write(
        root.join(".claude-plugin/plugin.json"),
        r#"{"name":"fixture"}"#,
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(
        root.join("hooks/hooks.json"),
        r#"{"modules":["./register.ts"]}"#,
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(root.join("hooks/register.ts"), source).map_err(|error| error.to_string())?;
    ModPlugin::discover(ModPluginRequest {
        id,
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
        session_id: "controls".into(),
        interactive: true,
        delegate: Arc::new(RejectHost),
        sink: Arc::new(|_| Box::pin(async {})),
    }
}

#[tokio::test]
async fn a_person_closing_a_game_clears_its_saved_activation()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let plugin = plugin(
        &home.path().join("plugin"),
        "fixture@test",
        r#"
export function register(on) {
  let path;
  on('session.start', async ($, event, next) => {
    path = (await $.env.get('HOME')) + '/.claude/mods/active-game';
    await $.fs.write(path, 'game');
    await $.ui.open({id:'game', title:'Game'});
    return next(event);
  });
  on('ui.close', async ($, event, next) => {
    const closed = await next(event);
    if (event.origin.kind === 'person') await $.fs.write(path, '');
    return closed;
  });
}
"#,
    )?;
    let session = ModSession::default();
    session.start(request(vec![plugin], home.path())).await?;
    let active = home
        .path()
        .join("claude-mods/home/.claude/mods/active-game");
    assert_eq!(std::fs::read_to_string(&active)?, "game");
    session.close("fixture@test", "game").await?;
    assert!(session.panes().await.is_empty());
    assert_eq!(std::fs::read_to_string(active)?, "");
    session.stop().await?;
    Ok(())
}

#[tokio::test]
async fn duplicate_plugin_names_keep_distinct_qualified_commands()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let source = "export function register(on) { on('session.start', async ($, event, next) => { await $.command.register({name:'answer'}); return next(event); }); }";
    let plugins = vec![
        plugin(&home.path().join("first"), "fixture@first", source)?,
        plugin(&home.path().join("second"), "fixture@second", source)?,
    ];
    let session = ModSession::default();
    session.start(request(plugins, home.path())).await?;
    let commands = session.commands().await;
    assert_eq!(commands.len(), 2);
    assert_eq!(commands[0].qualified_name, "fixture--first:answer");
    assert_eq!(commands[1].qualified_name, "fixture--second:answer");
    session.stop().await?;
    Ok(())
}
