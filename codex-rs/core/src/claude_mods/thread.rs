use crate::CodexThread;
use codex_protocol::claude_mods::ModCommand;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModPane;
use codex_protocol::claude_mods::ModRenderRequest;
use serde_json::Value;

impl CodexThread {
    pub async fn attach_claude_mod_ui(&self, interactive: bool) -> Result<(), String> {
        self.session
            .services
            .claude_mods
            .attach_ui(interactive)
            .await
    }
    pub async fn close_claude_mod(&self, plugin_id: &str, id: &str) -> Result<(), String> {
        self.session.services.claude_mods.close(plugin_id, id).await
    }
    pub async fn focus_claude_mod(
        &self,
        plugin_id: &str,
        id: &str,
        is_focused: bool,
        element: Option<&str>,
    ) -> Result<(), String> {
        self.session
            .services
            .claude_mods
            .focus(plugin_id, id, is_focused, element)
            .await
    }
    pub async fn claude_mod_commands(&self) -> Vec<ModCommand> {
        self.session.services.claude_mods.commands().await
    }
    pub async fn claude_mod_panes(&self) -> Vec<ModPane> {
        self.session.services.claude_mods.panes().await
    }
    pub async fn run_claude_mod_command(
        &self,
        plugin_id: &str,
        name: &str,
        arguments: &str,
    ) -> Result<String, String> {
        self.session
            .services
            .claude_mods
            .run_command(plugin_id, name, arguments)
            .await
    }
    pub async fn render_claude_mod(&self, request: ModRenderRequest) -> Result<ModNode, String> {
        self.session.services.claude_mods.render(request).await
    }
    pub async fn control_claude_mod(&self, callback_id: &str, value: Value) -> Result<(), String> {
        self.session
            .services
            .claude_mods
            .control(callback_id, value)
            .await
    }
}
