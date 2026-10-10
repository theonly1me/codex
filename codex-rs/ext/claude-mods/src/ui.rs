use crate::EventSink;
use crate::HostCall;
use crate::ModResult;
use crate::host::string;
use crate::ui_dimensions::DimensionRequest;
use crate::ui_dimensions::dimension;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::claude_mods::ModCommand;
use codex_protocol::claude_mods::ModPane;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use tokio::sync::Mutex;

pub(crate) struct Ui {
    pub(crate) state: Mutex<State>,
    pub(crate) interactive: std::sync::atomic::AtomicBool,
}

pub(crate) struct State {
    pub(crate) commands: BTreeMap<(String, String), ModCommand>,
    panes: BTreeMap<(String, String), ModPane>,
    config: BTreeMap<String, Value>,
}

impl Ui {
    pub(crate) fn new(interactive: bool) -> Self {
        Self {
            interactive: std::sync::atomic::AtomicBool::new(interactive),
            state: Mutex::new(State {
                commands: BTreeMap::new(),
                panes: BTreeMap::new(),
                config: BTreeMap::from([
                    ("theme".into(), json!("dark")),
                    ("verbose".into(), json!(false)),
                ]),
            }),
        }
    }

    pub(crate) async fn attach(&self, interactive: bool) {
        self.interactive
            .store(interactive, std::sync::atomic::Ordering::Release);
        for pane in self.state.lock().await.panes.values_mut() {
            pane.is_placed = interactive;
        }
    }

    pub(crate) async fn commands(&self) -> Vec<ModCommand> {
        self.state.lock().await.commands.values().cloned().collect()
    }
    pub(crate) async fn panes(&self) -> Vec<ModPane> {
        self.state.lock().await.panes.values().cloned().collect()
    }

    pub(crate) async fn call(&self, call: &HostCall, sink: &EventSink) -> ModResult<Value> {
        let first = call.arguments.first().unwrap_or(&Value::Null);
        let plugin_id = call.plugin.clone();
        let (event, result) = match call.method.as_str() {
            "command.register" => return self.register(call, sink).await,
            "command.list" => (
                None,
                serde_json::to_value(self.commands().await).map_err(|error| error.to_string())?,
            ),
            "ui.open" => {
                let id = string(&first["id"])?;
                let rows = dimension(DimensionRequest {
                    value: &first["rows"],
                    default: 20,
                    maximum: 256,
                })?;
                let columns = dimension(DimensionRequest {
                    value: &first["columns"],
                    default: 62,
                    maximum: 512,
                })?;
                let pane = ModPane {
                    plugin_id: plugin_id.clone(),
                    id: id.into(),
                    title: first["title"].as_str().unwrap_or(id).into(),
                    rows,
                    columns,
                    is_placed: self.interactive.load(std::sync::atomic::Ordering::Acquire),
                    focus: first["focus"].as_bool().unwrap_or(false),
                    close_on_escape: first["closeOnEscape"].as_bool().unwrap_or(true),
                };
                self.state
                    .lock()
                    .await
                    .panes
                    .insert((plugin_id, id.into()), pane.clone());
                (
                    Some(ClaudeModEvent::Open { pane }),
                    json!({"isPlaced":self.interactive.load(std::sync::atomic::Ordering::Acquire)}),
                )
            }
            "ui.close" => {
                let id = string(&first["id"])?;
                self.state
                    .lock()
                    .await
                    .panes
                    .remove(&(plugin_id.clone(), id.into()));
                (
                    Some(ClaudeModEvent::Close {
                        plugin_id,
                        id: id.into(),
                    }),
                    json!({"isClosed":true}),
                )
            }
            "ui.panes" => (
                None,
                serde_json::to_value(self.panes().await).map_err(|error| error.to_string())?,
            ),
            "ui.invalidate" => (Some(ClaudeModEvent::Invalidate { plugin_id }), Value::Null),
            "ui.status" => (
                Some(ClaudeModEvent::Status {
                    plugin_id,
                    text: first.as_str().map(str::to_string),
                }),
                Value::Null,
            ),
            "ui.toast" | "ui.log" | "ui.notice" => (
                Some(ClaudeModEvent::Toast {
                    plugin_id,
                    text: string(first)?.into(),
                }),
                Value::Null,
            ),
            "ui.blit" => {
                let cells = string(&first["cells"])?;
                if cells.len() > 2 * 1024 * 1024 {
                    return Err("mod raster exceeds 2 MiB".into());
                }
                (
                    Some(ClaudeModEvent::Raster {
                        plugin_id,
                        request_id: string(&first["requestId"])?.into(),
                        key: string(&first["key"])?.into(),
                        cells: cells.into(),
                    }),
                    json!({"isUpdated":self.interactive.load(std::sync::atomic::Ordering::Acquire)}),
                )
            }
            "config.list" => {
                let config = self
                    .state
                    .lock()
                    .await
                    .config
                    .iter()
                    .map(|(key, value)| json!({"key":key,"value":value}))
                    .collect::<Vec<_>>();
                (None, json!(config))
            }
            "config.set" => {
                let key = first
                    .as_str()
                    .or_else(|| first["key"].as_str())
                    .ok_or_else(|| "config.set requires a key".to_string())?
                    .to_string();
                if key != "theme" && key != "verbose" {
                    return Err(format!("unsupported mod config key {key}"));
                }
                let value = first
                    .get("value")
                    .or_else(|| call.arguments.get(1))
                    .cloned()
                    .ok_or_else(|| "config.set requires a value".to_string())?;
                if (key == "theme" && !value.is_string())
                    || (key == "verbose" && !value.is_boolean())
                {
                    return Err(format!("invalid mod config value for {key}"));
                }
                self.state
                    .lock()
                    .await
                    .config
                    .insert(key.clone(), value.clone());
                (
                    Some(ClaudeModEvent::Config {
                        key,
                        value: value.clone(),
                    }),
                    json!({"value":value}),
                )
            }
            _ => return Err(format!("unsupported mod UI API {}", call.method)),
        };
        if let Some(event) = event {
            sink(event).await;
        }
        Ok(result)
    }
}
