use crate::Dispatch;
use crate::ModResult;
use crate::ModSession;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModRenderRequest;
use serde_json::json;

impl ModSession {
    pub async fn render(&self, request: ModRenderRequest) -> ModResult<ModNode> {
        self.check()?;
        if !(1..=512).contains(&request.columns) || !(1..=256).contains(&request.rows) {
            return Err("mod viewport requires 1 to 512 columns and 1 to 256 rows".into());
        }
        let mut props = request.props;
        props.insert("bodyColumns".into(), json!(request.columns));
        props.insert("isFocused".into(), json!(request.is_focused));
        props.insert("isWorking".into(), json!(request.is_working));
        props.entry("placement").or_insert_with(|| json!("inline"));
        props.insert("scroll".into(), json!({"bodyRows":request.rows,"offset":0}));
        props.insert("maxRows".into(), json!(request.rows));
        let input = json!({"targetPluginId":props.get("targetPluginId"),"component":request.component,"requestId":request.request_id,"surface":"terminal","props":props,"viewport":{"columns":request.columns,"rows":request.rows,"isFullscreen":false}});
        let Some(runtime) = self.runtime() else {
            return Ok(ModNode::Empty);
        };
        let result = runtime.dispatch(Dispatch { name: "ui.render".into(), input }, |event| async move {
            if let Some(tree) = event["props"].get("nativeTree") { return Ok(tree.clone()); }
            if event["component"] == "Spinner" {
                return Ok(json!({"element":"Text","props":{},"children":[format!("{}{}", event["props"]["word"].as_str().unwrap_or_default(), event["props"]["suffix"].as_str().unwrap_or_default())]}));
            }
            if event["component"] == "PromptHint" {
                return Ok(json!({"element":"Text","props":{},"children":[event["props"]["tail"].as_str().unwrap_or_default()]}));
            }
            let text = event["props"]["text"].as_str().or_else(|| event["props"]["word"].as_str()).unwrap_or_default();
            Ok(json!({"element":"Text","props":{},"children":[text]}))
        }).await?;
        serde_json::from_value(result).map_err(|error| format!("invalid mod render tree: {error}"))
    }
}
