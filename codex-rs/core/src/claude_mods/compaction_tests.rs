use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::InstalledModLoader;
use codex_claude_mods_extension::ModHost;
use codex_claude_mods_extension::ModLoader;
use codex_claude_mods_extension::ModResult;
use codex_claude_mods_extension::ModSessionRequest;
use codex_claude_mods_extension::ModSource;
use codex_protocol::items::ContextCompactionItem;
use codex_protocol::items::TurnItem;
use futures::future::BoxFuture;
use serde_json::Value;
use std::sync::Arc;

struct RejectHost;

impl ModHost for RejectHost {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move { Err(format!("unexpected host API {}", call.method)) })
    }
}

#[tokio::test]
async fn native_compaction_items_notify_the_mod_through_upstream_contributors()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let plugin = home.path().join("plugin");
    std::fs::create_dir_all(plugin.join(".claude-plugin"))?;
    std::fs::write(
        plugin.join(".claude-plugin/plugin.json"),
        r#"{"name":"fixture","hooks":{"modules":["./register.js"]}}"#,
    )?;
    std::fs::write(
        plugin.join("register.js"),
        r#"
let count = 0;
export function register(on) {
  on('session.start', async ($, event, next) => {
    await $.command.register({name:'compactions'});
    return next(event);
  });
  on('classic.SessionStart', (engine, event, next) => {
    if (event.source === 'compact') count += 1;
    return next(event);
  });
  on('command.run', () => ({text:String(count)}));
}
"#,
    )?;
    let loader = InstalledModLoader {
        sources: vec![ModSource {
            id: "fixture@test".into(),
            root: plugin,
            enabled: true,
            active: true,
            options: Default::default(),
        }],
    };
    let (mut session, turn) = crate::session::tests::make_session_and_context().await;
    session.services.extensions = crate::session_hooks::install(
        &session.services.extensions,
        &session.services.thread_extension_data,
    );
    let session = Arc::new(session);
    super::compaction::bind(&session, &turn.session_source);
    let runtime = super::state(&session);
    runtime
        .start(ModSessionRequest {
            plugins: loader.load()?.plugins,
            codex_home: home.path().into(),
            cwd: home.path().display().to_string(),
            session_id: session.thread_id.to_string(),
            interactive: true,
            delegate: Arc::new(RejectHost),
            sink: Arc::new(|_| Box::pin(async {})),
        })
        .await?;
    assert_eq!(
        runtime
            .run_command("fixture@test", "compactions", "")
            .await?,
        "0"
    );
    session
        .emit_turn_item_completed(
            &turn,
            TurnItem::ContextCompaction(ContextCompactionItem::default()),
        )
        .await;
    assert_eq!(
        runtime
            .run_command("fixture@test", "compactions", "")
            .await?,
        "1"
    );
    runtime.stop().await?;
    Ok(())
}
