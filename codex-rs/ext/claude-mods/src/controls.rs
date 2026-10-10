use crate::ModHost;
use crate::ModResult;
use crate::ModSession;
use serde_json::Value;
use serde_json::json;

impl ModSession {
    pub async fn close(&self, plugin_id: &str, id: &str) -> ModResult<()> {
        let host = self
            .host()
            .ok_or_else(|| "mods are not loaded".to_string())?;
        host.call(crate::HostCall {
            plugin: plugin_id.into(),
            method: "ui.close".into(),
            arguments: vec![json!({"id":id,"origin":{"kind":"person"}})],
        })
        .await?;
        Ok(())
    }

    pub async fn focus(
        &self,
        plugin_id: &str,
        id: &str,
        is_focused: bool,
        element: Option<&str>,
    ) -> ModResult<()> {
        self.observe("ui.focus", json!({"id":id,"requestId":id,"component":"Pane","element":element,"isFocused":is_focused,"targetPluginId":plugin_id})).await?;
        Ok(())
    }

    pub async fn control(&self, callback_id: &str, value: Value) -> ModResult<()> {
        self.observe(
            "runtime.control",
            json!({"callbackId":callback_id,"value":value}),
        )
        .await?;
        Ok(())
    }
}
