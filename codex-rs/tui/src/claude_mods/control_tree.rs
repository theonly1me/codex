use codex_protocol::claude_mods::ModNode;
use serde_json::Value;

pub(super) struct Control {
    pub(super) callback: String,
    pub(super) hotkey: String,
    pub(super) input: bool,
    pub(super) element: String,
    pub(super) auto_focus: bool,
}

pub(super) fn collect(node: &ModNode, controls: &mut Vec<Control>) {
    match node {
        ModNode::Element(element) => {
            let input = element.element == "Input";
            let key = if input { "onSubmit" } else { "onPress" };
            if element.props.get("disabled").and_then(Value::as_bool) == Some(true) {
                return;
            }
            if let Some(callback) = element
                .props
                .get(key)
                .and_then(|callback| callback["callbackId"].as_str())
            {
                controls.push(Control {
                    callback: callback.into(),
                    element: element
                        .props
                        .get("key")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    hotkey: element
                        .props
                        .get("hotkey")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .into(),
                    input,
                    auto_focus: element.props.get("autoFocus").and_then(Value::as_bool)
                        == Some(true),
                });
            }
            for child in &element.children {
                collect(child, controls);
            }
        }
        ModNode::Children(children) => {
            for child in children {
                collect(child, controls);
            }
        }
        _ => {}
    }
}
