use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use serde_json::Map;
use serde_json::Value;

pub(super) fn style(props: &Map<String, Value>, inherited: Style) -> Style {
    let mut style = inherited;
    if let Some(color) = props.get("color").and_then(color) {
        style = style.fg(color);
    }
    if let Some(color) = props.get("backgroundColor").and_then(color) {
        style = style.bg(color);
    }
    for (key, modifier) in [
        ("bold", Modifier::BOLD),
        ("italic", Modifier::ITALIC),
        ("dimColor", Modifier::DIM),
    ] {
        if props.get(key).and_then(Value::as_bool) == Some(true) {
            style = style.add_modifier(modifier);
        }
    }
    style
}

pub(super) fn color(value: &Value) -> Option<Color> {
    value.as_str()?.parse().ok()
}
