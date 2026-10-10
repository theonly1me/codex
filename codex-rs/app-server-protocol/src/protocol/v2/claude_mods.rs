use crate::JsonSchema;
use crate::TS;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::claude_mods::ModCommand;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModPane;
use codex_protocol::claude_mods::ModRenderRequest;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModListParams {
    pub thread_id: String,
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModListResponse {
    pub commands: Vec<ModCommand>,
    pub panes: Vec<ModPane>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModCommandParams {
    pub thread_id: String,
    pub plugin_id: String,
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModCommandResponse {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModRenderParams {
    pub thread_id: String,
    pub render: ModRenderRequest,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModRenderResponse {
    pub tree: ModNode,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModControlParams {
    pub thread_id: String,
    pub interaction: ClaudeModInteraction,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export_to = "v2/")]
pub enum ClaudeModInteraction {
    Callback {
        callback_id: String,
        #[ts(type = "unknown")]
        value: Value,
    },
    Close {
        plugin_id: String,
        id: String,
    },
    Focus {
        plugin_id: String,
        id: String,
        is_focused: bool,
        element: Option<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[ts(export_to = "v2/")]
pub struct ClaudeModControlResponse {}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ClaudeModChangedNotification {
    pub thread_id: String,
    pub event: ClaudeModEvent,
}
