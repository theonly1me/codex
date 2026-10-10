use codex_claude_mod_runtime::ModPluginRequest;
use codex_claude_mods_extension::*;
use futures::future::BoxFuture;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

struct RejectHost;

impl ModHost for RejectHost {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move { Err(format!("unexpected host API {}", call.method)) })
    }
}

fn discover(root: &Path) -> ModResult<ModPlugin> {
    ModPlugin::discover(ModPluginRequest {
        id: "fixture@test",
        root,
        options: &Map::new(),
    })?
    .ok_or_else(|| "fixture has no mod".into())
}

#[tokio::test]
async fn reactive_state_survives_reload_and_resets_without_deleting_the_persistent_store()
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
import {atom, read, update} from 'claude-code';
const count = atom({plugin:'fixture', key:'count'}, 0);
export function register(on) {
  on('session.start', async ($, event, next) => {
    await $.command.register({name:'count'});
    return next(event);
  });
  on('command.run', async ($, event) => {
    if (event.args === 'add') await $.store.set('count', await update($, count, value => value + 1));
    return {text:JSON.stringify({session:await read($, count), saved:await $.store.get('count')})};
  });
}
"#;
    std::fs::write(&module, source)?;
    let events = Arc::new(Mutex::new(Vec::new()));
    let request = |plugin| {
        let events = Arc::clone(&events);
        ModSessionRequest {
            plugins: vec![plugin],
            codex_home: home.path().into(),
            cwd: home.path().display().to_string(),
            session_id: "state".into(),
            interactive: true,
            delegate: Arc::new(RejectHost),
            sink: Arc::new(move |event| {
                let events = Arc::clone(&events);
                Box::pin(async move {
                    events.lock().unwrap().push(event);
                })
            }),
        }
    };
    let session = ModSession::default();
    session.start(request(discover(&root)?)).await?;
    events.lock().unwrap().clear();
    let count: Value =
        serde_json::from_str(&session.run_command("fixture@test", "count", "add").await?)?;
    assert_eq!(count, json!({"session":1,"saved":1}));
    assert!(events.lock().unwrap().iter().any(|event| matches!(
        event,
        codex_protocol::claude_mods::ClaudeModEvent::Invalidate { .. }
    )));
    std::fs::write(&module, format!("{source}\n"))?;
    session.start(request(discover(&root)?)).await?;
    let count: Value =
        serde_json::from_str(&session.run_command("fixture@test", "count", "").await?)?;
    assert_eq!(count, json!({"session":1,"saved":1}));
    session.stop().await?;
    session.start(request(discover(&root)?)).await?;
    let count: Value =
        serde_json::from_str(&session.run_command("fixture@test", "count", "").await?)?;
    assert_eq!(count, json!({"session":0,"saved":1}));
    session.stop().await?;
    Ok(())
}
