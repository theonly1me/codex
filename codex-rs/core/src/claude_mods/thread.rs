use crate::CodexThread;
use codex_protocol::claude_mods::ModCommand;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModPane;
use codex_protocol::claude_mods::ModRenderRequest;
use serde_json::Value;

impl CodexThread {
    pub async fn attach_claude_mod_ui(&self, interactive: bool) -> Result<(), String> {
        super::state(&self.session).attach_ui(interactive).await
    }
    pub async fn close_claude_mod(&self, plugin_id: &str, id: &str) -> Result<(), String> {
        super::state(&self.session).close(plugin_id, id).await
    }
    pub async fn focus_claude_mod(
        &self,
        plugin_id: &str,
        id: &str,
        is_focused: bool,
        element: Option<&str>,
    ) -> Result<(), String> {
        super::state(&self.session)
            .focus(plugin_id, id, is_focused, element)
            .await
    }
    pub async fn claude_mod_commands(&self) -> Vec<ModCommand> {
        super::state(&self.session).commands().await
    }
    pub async fn claude_mod_panes(&self) -> Vec<ModPane> {
        super::state(&self.session).panes().await
    }
    pub async fn run_claude_mod_command(
        &self,
        plugin_id: &str,
        name: &str,
        arguments: &str,
    ) -> Result<String, String> {
        super::state(&self.session)
            .run_command(plugin_id, name, arguments)
            .await
    }
    pub async fn render_claude_mod(&self, request: ModRenderRequest) -> Result<ModNode, String> {
        super::state(&self.session).render(request).await
    }
    pub async fn control_claude_mod(&self, callback_id: &str, value: Value) -> Result<(), String> {
        super::state(&self.session)
            .control(callback_id, value)
            .await
    }
}
