use super::host::CoreHost;
use crate::config::Config;
use crate::session::session::Session;
use codex_claude_mods_extension::ModSessionRequest;
use codex_core_plugins::load_claude_mod_plugins;
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
    let plugins = match load_claude_mod_plugins(&loaded, &input, &config.codex_home) {
        Ok(plugins) => plugins,
        Err(error) => {
            session.services.claude_mods.discovery_failed(error.clone());
            return Err(error);
        }
    };
    let enabled = loaded
        .plugins()
        .iter()
        .filter(|plugin| plugin.enabled)
        .map(|plugin| plugin.config_name.clone())
        .collect::<Vec<_>>();
    session
        .services
        .claude_mods
        .validate_reload(&enabled, &plugins)?;
    if plugins.is_empty()
        && session.services.claude_mods.runtime().is_none()
        && session.services.claude_mods.check().is_ok()
    {
        return Ok(());
    }
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
    session
        .services
        .claude_mods
        .start(ModSessionRequest {
            plugins,
            codex_home: config.codex_home.to_path_buf(),
            cwd,
            session_id: session.thread_id.to_string(),
            interactive: matches!(source, SessionSource::Cli),
            delegate: Arc::new(CoreHost {
                session: Arc::downgrade(session),
            }),
            sink,
        })
        .await
}
