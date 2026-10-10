use super::invalid_request;
use super::thread_processor::ThreadRequestProcessor;
use codex_app_server_protocol::*;
use codex_protocol::ThreadId;

impl ThreadRequestProcessor {
    async fn mod_thread(
        &self,
        thread_id: &str,
    ) -> std::result::Result<std::sync::Arc<codex_core::CodexThread>, JSONRPCErrorError> {
        let id =
            ThreadId::from_string(thread_id).map_err(|error| invalid_request(error.to_string()))?;
        self.thread_manager
            .get_thread(id)
            .await
            .map_err(|error| invalid_request(error.to_string()))
    }

    pub(crate) async fn claude_mod_list(
        &self,
        params: ClaudeModListParams,
    ) -> std::result::Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread = self.mod_thread(&params.thread_id).await?;
        thread
            .attach_claude_mod_ui(params.interactive)
            .await
            .map_err(invalid_request)?;
        Ok(Some(
            ClaudeModListResponse {
                commands: thread.claude_mod_commands().await,
                panes: thread.claude_mod_panes().await,
            }
            .into(),
        ))
    }

    pub(crate) async fn claude_mod_command(
        &self,
        params: ClaudeModCommandParams,
    ) -> std::result::Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread = self.mod_thread(&params.thread_id).await?;
        let text = thread
            .run_claude_mod_command(&params.plugin_id, &params.name, &params.arguments)
            .await
            .map_err(invalid_request)?;
        Ok(Some(ClaudeModCommandResponse { text }.into()))
    }

    pub(crate) async fn claude_mod_render(
        &self,
        params: ClaudeModRenderParams,
    ) -> std::result::Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread = self.mod_thread(&params.thread_id).await?;
        let tree = thread
            .render_claude_mod(params.render)
            .await
            .map_err(invalid_request)?;
        Ok(Some(ClaudeModRenderResponse { tree }.into()))
    }

    pub(crate) async fn claude_mod_control(
        &self,
        params: ClaudeModControlParams,
    ) -> std::result::Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread = self.mod_thread(&params.thread_id).await?;
        match params.interaction {
            ClaudeModInteraction::Callback { callback_id, value } => {
                thread.control_claude_mod(&callback_id, value).await
            }
            ClaudeModInteraction::Close { plugin_id, id } => {
                thread.close_claude_mod(&plugin_id, &id).await
            }
            ClaudeModInteraction::Focus {
                plugin_id,
                id,
                is_focused,
                element,
            } => {
                thread
                    .focus_claude_mod(&plugin_id, &id, is_focused, element.as_deref())
                    .await
            }
        }
        .map_err(invalid_request)?;
        Ok(Some(ClaudeModControlResponse {}.into()))
    }
}
