use super::App;
use crate::app_server_session::AppServerSession;
use crate::claude_mods::history::ModHistoryCell;
use crate::history_cell::HistoryCell;
use codex_app_server_protocol::ClaudeModRenderParams;
use codex_app_server_protocol::ClaudeModRenderResponse;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_protocol::claude_mods::ModRenderRequest;
use serde_json::json;

impl App {
    pub(super) async fn decorate_mod_history(
        &mut self,
        server: &AppServerSession,
        cell: Box<dyn HistoryCell>,
    ) -> Box<dyn HistoryCell> {
        if self.chat_widget.mod_ui.commands.is_empty() {
            return cell;
        }
        let Some(component) = crate::claude_mods::history::component(cell.as_ref()) else {
            return cell;
        };
        let Some(thread_id) = self.chat_widget.thread_id() else {
            return cell;
        };
        let text = cell
            .raw_lines()
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let props = json!({"text":text,"origin":{"kind":"composer"},"isExpanded":false,"isFirstOfReply":true,"word":"Completed","tool":"CodexTool","input":{},"output":{"text":text},"isRunning":false,"isErrored":false,"isInterrupted":false,"nativeTree":{"element":"Native"}});
        let Some(mut props) = props.as_object().cloned() else {
            return cell;
        };
        if let Some(separator) = cell
            .as_any()
            .downcast_ref::<crate::history_cell::FinalMessageSeparator>()
        {
            props.insert("durationMs".into(), json!(separator.duration_ms()));
        }
        if let Some(exec) = cell.as_any().downcast_ref::<crate::exec_cell::ExecCell>()
            && let Some(extra) = exec.mod_props().as_object()
        {
            props.extend(extra.clone());
        }
        let (columns, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        if component == "ToolUse" {
            props.insert(
                "nativeTree".into(),
                json!({"element":"Native","props":{"slot":"use"}}),
            );
        }
        let result = server
            .request_handle()
            .request_typed::<ClaudeModRenderResponse>(ClientRequest::ClaudeModRender {
                request_id: RequestId::String(format!("mod-history-{}", uuid::Uuid::new_v4())),
                params: ClaudeModRenderParams {
                    thread_id: thread_id.to_string(),
                    render: ModRenderRequest {
                        component: component.into(),
                        request_id: format!("history-{}", uuid::Uuid::new_v4()),
                        columns,
                        rows,
                        is_focused: false,
                        is_working: false,
                        props: props.clone(),
                    },
                },
            })
            .await;
        match result {
            Ok(response) if component == "ToolUse" => {
                props.insert(
                    "nativeTree".into(),
                    json!({"element":"Native","props":{"slot":"result"}}),
                );
                let result = server
                    .request_handle()
                    .request_typed::<ClaudeModRenderResponse>(ClientRequest::ClaudeModRender {
                        request_id: RequestId::String(format!(
                            "mod-result-{}",
                            uuid::Uuid::new_v4()
                        )),
                        params: ClaudeModRenderParams {
                            thread_id: thread_id.to_string(),
                            render: ModRenderRequest {
                                component: "ToolResult".into(),
                                request_id: format!("result-{}", uuid::Uuid::new_v4()),
                                columns,
                                rows,
                                is_focused: false,
                                is_working: false,
                                props,
                            },
                        },
                    })
                    .await;
                match result {
                    Ok(result) if is_native(&response.tree) && is_native(&result.tree) => cell,
                    Ok(result) => Box::new(ModHistoryCell {
                        native: cell,
                        tree: codex_protocol::claude_mods::ModNode::Children(vec![
                            response.tree,
                            result.tree,
                        ]),
                    }),
                    Err(error) => {
                        self.chat_widget
                            .add_error_message(format!("Mod ToolResult render failed: {error}"));
                        cell
                    }
                }
            }
            Ok(response) if is_native(&response.tree) => cell,
            Ok(response) => Box::new(ModHistoryCell {
                native: cell,
                tree: response.tree,
            }),
            Err(error) => {
                self.chat_widget
                    .add_error_message(format!("Mod {component} render failed: {error}"));
                cell
            }
        }
    }
}

fn is_native(node: &codex_protocol::claude_mods::ModNode) -> bool {
    matches!(node, codex_protocol::claude_mods::ModNode::Element(element) if element.element == "Native")
}
