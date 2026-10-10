use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModCommand {
    pub plugin_id: String,
    pub name: String,
    pub qualified_name: String,
    pub description: String,
    pub argument_hint: Option<String>,
    pub immediate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModPane {
    pub plugin_id: String,
    pub id: String,
    pub title: String,
    pub rows: u16,
    pub columns: u16,
    pub is_placed: bool,
    pub focus: bool,
    pub close_on_escape: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(untagged)]
pub enum ModNode {
    Text(String),
    Number(f64),
    Element(ModElement),
    Children(Vec<ModNode>),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
pub struct ModElement {
    pub element: String,
    #[serde(default)]
    pub plugin: Option<String>,
    #[serde(default)]
    #[ts(type = "Record<string, unknown>")]
    pub props: serde_json::Map<String, Value>,
    #[serde(default)]
    pub children: Vec<ModNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ModRenderRequest {
    pub component: String,
    pub request_id: String,
    pub columns: u16,
    pub rows: u16,
    pub is_focused: bool,
    pub is_working: bool,
    #[serde(default)]
    #[ts(type = "Record<string, unknown>")]
    pub props: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ClaudeModEvent {
    Copy {
        text: String,
    },
    ThemePicker,
    Command {
        command: ModCommand,
    },
    Open {
        pane: ModPane,
    },
    Close {
        plugin_id: String,
        id: String,
    },
    Invalidate {
        plugin_id: String,
    },
    Raster {
        plugin_id: String,
        request_id: String,
        key: String,
        cells: String,
    },
    Status {
        plugin_id: String,
        text: Option<String>,
    },
    Toast {
        plugin_id: String,
        text: String,
    },
    PromptFill {
        plugin_id: String,
        text: String,
    },
    Config {
        key: String,
        #[ts(type = "unknown")]
        value: Value,
    },
    Error {
        plugin_id: String,
        message: String,
    },
}
