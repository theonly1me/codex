use super::App;
use crate::app_event::AppEvent;
use crate::app_server_session::AppServerSession;
use codex_app_server_protocol::*;
use codex_protocol::claude_mods::ModRenderRequest;
use serde_json::json;

impl App {
    pub(super) fn refresh_claude_mods(&mut self, server: &AppServerSession) {
        let Some(thread_id) = self.chat_widget.thread_id().map(|id| id.to_string()) else {
            self.chat_widget.mod_ui.pending = false;
            return;
        };
        let handle = server.request_handle();
        let sender = self.app_event_tx.clone();
        let working = self.chat_widget.mod_is_working();
        let active_pane = self.chat_widget.mod_ui.active_pane.clone();
        let survey = self.chat_widget.mod_has_survey();
        let spinner_word = self.chat_widget.mod_spinner_word();
        let (columns, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        tokio::spawn(async move {
            let result = async {
                let catalog = handle
                    .request_typed::<ClaudeModListResponse>(ClientRequest::ClaudeModList {
                        request_id: request_id(),
                        params: ClaudeModListParams {
                            thread_id: thread_id.clone(),
                            interactive: true,
                        },
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                let mut requests = vec![(
                    "AbovePrompt".to_string(),
                    "AbovePrompt".to_string(),
                    json!({"hasSurvey":survey}),
                )];
                requests.push((
                    "PromptHint".into(),
                    "PromptHint".into(),
                    json!({"hint":"","tail":"","isDraft":false,"isWorking":working}),
                ));
                if working {
                    requests.push((
                        "Spinner".into(),
                        "Spinner".into(),
                        json!({"word":spinner_word,"message":null,"suffix":"","mode":"responding"}),
                    ));
                }
                for pane in &catalog.panes {
                    requests.push((
                        crate::claude_mods::pane_request_id(pane),
                        "Pane".into(),
                        json!({"id":pane.id,"placement":"dock","targetPluginId":pane.plugin_id}),
                    ));
                }
                let mut trees = Vec::new();
                for (id, component, props) in requests {
                    let props = props
                        .as_object()
                        .cloned()
                        .ok_or_else(|| "invalid mod render properties".to_string())?;
                    let response = handle
                        .request_typed::<ClaudeModRenderResponse>(ClientRequest::ClaudeModRender {
                            request_id: request_id(),
                            params: ClaudeModRenderParams {
                                thread_id: thread_id.clone(),
                                render: ModRenderRequest {
                                    component: component.clone(),
                                    request_id: if component == "Pane" {
                                        props
                                            .get("id")
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or(&id)
                                            .to_string()
                                    } else {
                                        id.clone()
                                    },
                                    columns,
                                    rows: rows.saturating_sub(6).max(1),
                                    is_focused: active_pane.as_ref() == Some(&id),
                                    is_working: working,
                                    props,
                                },
                            },
                        })
                        .await
                        .map_err(|error| error.to_string())?;
                    trees.push((id, response.tree));
                }
                Ok((catalog, trees))
            }
            .await;
            sender.send(AppEvent::ClaudeModFrames { thread_id, result });
        });
    }

    pub(super) fn run_claude_mod_command(
        &self,
        server: &AppServerSession,
        plugin_id: String,
        name: String,
        arguments: String,
    ) {
        let Some(thread_id) = self.chat_widget.thread_id().map(|id| id.to_string()) else {
            return;
        };
        let handle = server.request_handle();
        let sender = self.app_event_tx.clone();
        tokio::spawn(async move {
            let result = handle
                .request_typed::<ClaudeModCommandResponse>(ClientRequest::ClaudeModCommand {
                    request_id: request_id(),
                    params: ClaudeModCommandParams {
                        thread_id: thread_id.clone(),
                        plugin_id,
                        name,
                        arguments,
                    },
                })
                .await
                .map(|response| response.text)
                .map_err(|error| error.to_string());
            sender.send(AppEvent::ClaudeModCommandResult { thread_id, result });
        });
    }

    pub(super) fn control_claude_mod(
        &self,
        server: &AppServerSession,
        interaction: ClaudeModInteraction,
    ) {
        let Some(thread_id) = self.chat_widget.thread_id().map(|id| id.to_string()) else {
            return;
        };
        let handle = server.request_handle();
        let sender = self.app_event_tx.clone();
        tokio::spawn(async move {
            let result = handle
                .request_typed::<ClaudeModControlResponse>(ClientRequest::ClaudeModControl {
                    request_id: request_id(),
                    params: ClaudeModControlParams {
                        thread_id: thread_id.clone(),
                        interaction,
                    },
                })
                .await
                .map(|_| String::new())
                .map_err(|error| error.to_string());
            sender.send(AppEvent::ClaudeModCommandResult { thread_id, result });
        });
    }
}

fn request_id() -> RequestId {
    RequestId::String(format!("mod-{}", uuid::Uuid::new_v4()))
}
