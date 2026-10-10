use crate::Dispatch;
use crate::ModHost;
use crate::ModResult;
use crate::ModRuntime;
use crate::ModSession;
use crate::ModSessionRequest;
use crate::host::Host;
use crate::storage::Storage;
use crate::ui::Ui;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;

impl ModSession {
    pub async fn start(&self, request: ModSessionRequest) -> ModResult<()> {
        let _activation = self
            .activation
            .acquire()
            .await
            .map_err(|_| "mod activation stopped".to_string())?;
        let signature = serde_json::to_string(
            &request
                .plugins
                .iter()
                .map(|plugin| {
                    (
                        &plugin.id,
                        &plugin.root,
                        &plugin.module,
                        &plugin.additional_modules,
                        &plugin.source_signature,
                        &plugin.options,
                    )
                })
                .collect::<Vec<_>>(),
        )
        .map_err(|error| error.to_string())?;
        if self.state().signature == signature && self.check().is_ok() {
            return Ok(());
        }
        let interactive = request.interactive
            || self.host().is_some_and(|host| {
                host.ui
                    .interactive
                    .load(std::sync::atomic::Ordering::Acquire)
            });
        if let Some(runtime) = self.runtime() {
            if let Err(message) = runtime
                .observe(Dispatch {
                    name: "session.end".into(),
                    input: json!({}),
                })
                .await
            {
                (request.sink)(codex_protocol::claude_mods::ClaudeModEvent::Error {
                    plugin_id: "runtime".into(),
                    message: format!("Mod cleanup failed during reload: {message}"),
                })
                .await;
            }
            runtime.shutdown();
        }
        if let Some(host) = self.host() {
            for plugin_id in host.plugins.keys() {
                (request.sink)(codex_protocol::claude_mods::ClaudeModEvent::Status {
                    plugin_id: plugin_id.clone(),
                    text: None,
                })
                .await;
            }
        }
        let old_panes = self.panes().await;
        for pane in old_panes {
            (request.sink)(codex_protocol::claude_mods::ClaudeModEvent::Close {
                plugin_id: pane.plugin_id,
                id: pane.id,
            })
            .await;
        }
        *self.state() = super::session::State {
            signature,
            ..Default::default()
        };
        (request.sink)(codex_protocol::claude_mods::ClaudeModEvent::Invalidate {
            plugin_id: "runtime".into(),
        })
        .await;
        if request.plugins.is_empty() {
            return Ok(());
        }
        let host = Arc::new(Host {
            reactive: Arc::clone(&self.reactive),
            runtime: OnceCell::new(),
            plugins: request
                .plugins
                .iter()
                .cloned()
                .map(|plugin| (plugin.id.clone(), plugin))
                .collect::<HashMap<_, _>>(),
            storage: match Storage::new(&request.codex_home) {
                Ok(storage) => storage,
                Err(error) => {
                    self.state().failure = Some(error.clone());
                    return Err(error);
                }
            },
            ui: Ui::new(interactive),
            delegate: request.delegate,
            sink: request.sink,
        });
        for plugin in &request.plugins {
            if !plugin.remapped_models.is_empty() {
                (host.sink)(codex_protocol::claude_mods::ClaudeModEvent::Toast {
                    plugin_id: plugin.id.clone(),
                    text: format!(
                        "{}: {} use the active Codex model",
                        plugin.name,
                        plugin.remapped_models.join(", ")
                    ),
                })
                .await;
            }
        }
        let runtime =
            match ModRuntime::start(request.plugins, Arc::clone(&host) as Arc<dyn ModHost>).await {
                Ok(runtime) => runtime,
                Err(error) => {
                    self.state().failure = Some(error.clone());
                    return Err(error);
                }
            };
        host.runtime
            .set(runtime.downgrade())
            .map_err(|_| "mod runtime already linked".to_string())?;
        {
            let mut state = self.state();
            state.host = Some(host);
            state.runtime = Some(runtime);
            state.start_event = json!({"cwd":request.cwd,"sessionId":request.session_id,"isInteractive":interactive});
        }
        let initialized = async {
            self.observe("session.start", json!({"cwd":request.cwd,"sessionId":request.session_id,"isInteractive":interactive})).await?;
            self.observe("classic.SessionStart", json!({"cwd":request.cwd,"session_id":request.session_id,"source":"startup"})).await?;
            Ok::<(), String>(())
        }.await;
        if let Err(error) = initialized {
            self.discovery_failed(error.clone());
            return Err(error);
        }
        Ok(())
    }
}
