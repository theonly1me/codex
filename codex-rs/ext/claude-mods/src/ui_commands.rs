use crate::EventSink;
use crate::HostCall;
use crate::ModResult;
use crate::host::string;
use crate::ui::Ui;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::claude_mods::ModCommand;
use serde_json::Value;
use serde_json::json;

impl Ui {
    pub(crate) async fn register(&self, call: &HostCall, sink: &EventSink) -> ModResult<Value> {
        let first = call.arguments.first().unwrap_or(&Value::Null);
        let plugin_id = call.plugin.clone();
        let (event, result) = {
            let name = string(&first["name"])?;
            if name.is_empty()
                || !name.bytes().all(|character| {
                    character.is_ascii_alphanumeric() || b"-_".contains(&character)
                })
            {
                return Err(
                    "mod command names require letters, numbers, hyphens, or underscores".into(),
                );
            }
            let namespace = plugin_id.split('@').next().unwrap_or(&plugin_id);
            let command = ModCommand {
                plugin_id: plugin_id.clone(),
                name: name.into(),
                qualified_name: format!("{namespace}:{name}"),
                description: first["description"].as_str().unwrap_or_default().into(),
                argument_hint: first["argumentHint"].as_str().map(str::to_string),
                immediate: first["immediate"].as_bool().unwrap_or(false),
            };
            let conflicts = {
                let mut state = self.state.lock().await;
                state
                    .commands
                    .insert((plugin_id.clone(), name.into()), command.clone());
                let duplicate_namespace = state.commands.values().any(|entry| {
                    entry.plugin_id != plugin_id && entry.qualified_name == command.qualified_name
                });
                if duplicate_namespace {
                    for entry in state
                        .commands
                        .values_mut()
                        .filter(|entry| entry.name == name)
                    {
                        entry.qualified_name =
                            format!("{}:{}", entry.plugin_id.replace('@', "--"), name);
                    }
                }
                state
                    .commands
                    .values()
                    .filter(|entry| entry.name == name)
                    .cloned()
                    .collect::<Vec<_>>()
            };
            if conflicts.len() > 1 {
                for entry in &conflicts {
                    sink(ClaudeModEvent::Command {
                        command: entry.clone(),
                    })
                    .await;
                }
                sink(ClaudeModEvent::Toast {
                    plugin_id: plugin_id.clone(),
                    text: format!(
                        "Command /{name} is shared. Use {}",
                        conflicts
                            .iter()
                            .map(|entry| format!("/{}", entry.qualified_name))
                            .collect::<Vec<_>>()
                            .join(" or ")
                    ),
                })
                .await;
            }
            let command = conflicts
                .into_iter()
                .find(|entry| entry.plugin_id == plugin_id)
                .ok_or_else(|| "registered mod command disappeared".to_string())?;
            (
                Some(ClaudeModEvent::Command { command }),
                json!({"isRegistered":true}),
            )
        };
        if let Some(event) = event {
            sink(event).await;
        }
        Ok(result)
    }
}
