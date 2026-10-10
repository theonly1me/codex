use super::host::CoreHost;
use crate::config::Config;
use crate::session::session::Session;
use codex_claude_mods_extension::InstalledModLoader;
use codex_claude_mods_extension::ModSessionContext;
use codex_claude_mods_extension::ModSource;
use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::SessionSource;
use std::sync::Arc;

pub(crate) async fn start(
    session: &Arc<Session>,
    config: &Config,
    source: &SessionSource,
    disabled: &[String],
) -> Result<(), String> {
    if matches!(source, SessionSource::Internal(_)) {
        return Ok(());
    }
    let input = config.plugins_config_input();
    let loaded = session
        .services
        .plugins_manager
        .plugins_for_config(&input)
        .await
        .without_plugins(disabled);
    let loader = InstalledModLoader {
        sources: loaded
            .plugins()
            .iter()
            .map(|plugin| ModSource {
                id: plugin.config_name.clone(),
                root: plugin.root.to_path_buf(),
                enabled: plugin.enabled,
                active: plugin.is_active(),
                options: input
                    .plugins
                    .plugins
                    .get(&plugin.config_name)
                    .map(|configuration| configuration.options.clone())
                    .unwrap_or_default(),
            })
            .collect(),
    };
    super::state(session)
        .load(&loader, || context(session, config, source))
        .await
}

async fn context(
    session: &Arc<Session>,
    config: &Config,
    source: &SessionSource,
) -> Result<ModSessionContext, String> {
    let weak = Arc::downgrade(session);
    let sink: codex_claude_mods_extension::EventSink = Arc::new(move |event| {
        let weak = weak.clone();
        Box::pin(async move {
            if let Some(session) = weak.upgrade() {
                session
                    .send_event_raw(Event {
                        id: "claude-mods".into(),
                        msg: EventMsg::ClaudeMod(event),
                    })
                    .await;
            }
        })
    });
    let cwd = super::host::step(session)
        .await?
        .environments()
        .first()
        .ok_or_else(|| "mod has no environment".to_string())?
        .0
        .cwd()
        .to_abs_path()
        .map_err(|error| error.to_string())?
        .as_path()
        .display()
        .to_string();
    Ok(ModSessionContext {
        codex_home: config.codex_home.to_path_buf(),
        cwd,
        session_id: session.thread_id.to_string(),
        interactive: matches!(source, SessionSource::Cli),
        delegate: Arc::new(CoreHost {
            session: Arc::downgrade(session),
        }),
        sink,
    })
}
