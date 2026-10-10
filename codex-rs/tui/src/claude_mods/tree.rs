use super::ModUi;
use codex_protocol::claude_mods::ModNode;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use serde_json::Value;

pub(super) fn lines(
    node: &ModNode,
    ui: &ModUi,
    request_id: &str,
    width: u16,
    inherited: Style,
) -> Vec<Line<'static>> {
    match node {
        ModNode::Empty => Vec::new(),
        ModNode::Text(text) => text
            .lines()
            .map(|line| Line::from(Span::styled(line.to_string(), inherited)))
            .collect(),
        ModNode::Number(number) => vec![Line::from(Span::styled(number.to_string(), inherited))],
        ModNode::Children(children) => children
            .iter()
            .flat_map(|child| lines(child, ui, request_id, width, inherited))
            .collect(),
        ModNode::Element(element) => {
            let props = &element.props;
            let style = super::style::style(props, inherited);
            let string = |key| props.get(key).and_then(Value::as_str).unwrap_or_default();
            if element.element == "Native" {
                let mut native = ui.native_lines.clone().unwrap_or_default();
                match string("slot") {
                    "use" => native.truncate(1),
                    "result" if !native.is_empty() => {
                        native.remove(0);
                    }
                    _ => {}
                }
                return crate::wrapping::word_wrap_lines(&native, usize::from(width))
                    .into_iter()
                    .collect();
            }
            if element.element == "Raster" {
                let cells = ui
                    .rasters
                    .get(&(request_id.into(), string("key").into()))
                    .map(String::as_str)
                    .unwrap_or_else(|| string("cells"));
                let columns = props
                    .get("columns")
                    .and_then(Value::as_u64)
                    .unwrap_or(u64::from(width)) as usize;
                let rows = props.get("rows").and_then(Value::as_u64).unwrap_or(0) as usize;
                return super::raster::lines(cells, columns, rows)
                    .unwrap_or_else(|error| vec![Line::from(error)]);
            }
            if element.element == "Button" {
                let label = string("label");
                let hotkey = string("hotkey");
                let marker = if ui.focused_callback().as_deref()
                    == props
                        .get("onPress")
                        .and_then(|callback| callback["callbackId"].as_str())
                {
                    ">"
                } else {
                    " "
                };
                return vec![Line::from(Span::styled(
                    format!(
                        "{marker}[{label}]{}",
                        if hotkey.is_empty() {
                            String::new()
                        } else {
                            format!(" ({hotkey})")
                        }
                    ),
                    style,
                ))];
            }
            if element.element == "Input" {
                return vec![Line::from(Span::styled(
                    format!(
                        "> {}",
                        if ui.input.is_empty() {
                            string("placeholder")
                        } else {
                            &ui.input
                        }
                    ),
                    style,
                ))];
            }
            if element.element == "Markdown" {
                let mut lines = Vec::new();
                let cwd = None;
                crate::markdown::append_markdown(
                    string("text"),
                    Some(usize::from(width)),
                    cwd,
                    &mut lines,
                );
                for line in &mut lines {
                    line.style = line.style.patch(style);
                }
                return lines;
            }
            if element.element == "Code" {
                return props
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| string("text"))
                    .lines()
                    .map(|line| Line::from(Span::styled(line.to_string(), style)))
                    .collect();
            }
            super::layout::lines(element, ui, request_id, width, style)
        }
    }
}
