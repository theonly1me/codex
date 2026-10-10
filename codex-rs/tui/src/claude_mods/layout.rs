use super::ModUi;
use codex_protocol::claude_mods::ModElement;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use serde_json::Map;
use serde_json::Value;

const NO_SPACING: usize = 0;

pub(super) fn lines(
    element: &ModElement,
    ui: &ModUi,
    request_id: &str,
    width: u16,
    style: Style,
) -> Vec<Line<'static>> {
    let properties = &element.props;
    let requested = dimension(properties, "width", usize::from(width)).min(usize::from(width));
    let padding = dimension(properties, "padding", NO_SPACING).min(20);
    let horizontal = dimension(properties, "paddingX", padding).min(20);
    let vertical = dimension(properties, "paddingY", padding).min(20);
    let border = properties.get("borderStyle").and_then(Value::as_str);
    let border_width = usize::from(border.is_some()) * 2;
    let inner = requested
        .saturating_sub(horizontal * 2 + border_width)
        .max(1);
    let children = element
        .children
        .iter()
        .map(|child| {
            super::tree::lines(
                child,
                ui,
                request_id,
                u16::try_from(inner).unwrap_or(width),
                style,
            )
        })
        .collect::<Vec<_>>();
    let row = properties.get("flexDirection").and_then(Value::as_str) == Some("row")
        || element.element == "Text";
    let gap = dimension(
        properties,
        "gap",
        dimension(properties, "columnGap", NO_SPACING),
    )
    .min(20);
    let wrapping = properties.get("flexWrap").and_then(Value::as_str) == Some("wrap");
    let mut output = if row {
        row_lines(children, gap, inner, wrapping)
    } else {
        children.into_iter().flatten().collect()
    };
    output = crate::wrapping::word_wrap_lines(&output, inner)
        .into_iter()
        .collect();
    let centered = properties.get("justifyContent").and_then(Value::as_str) == Some("center");
    for line in &mut output {
        line.style = line.style.patch(style);
        let left = if centered {
            inner.saturating_sub(line.width()) / 2
        } else {
            0
        };
        if left + horizontal > 0 {
            line.spans
                .insert(0, Span::styled(" ".repeat(left + horizontal), style));
        }
        if border.is_some() || properties.contains_key("backgroundColor") {
            line.spans.push(Span::styled(
                " ".repeat(requested.saturating_sub(border_width + line.width())),
                style,
            ));
        }
    }
    for _ in 0..vertical {
        output.insert(
            0,
            Line::from(Span::styled(
                " ".repeat(requested.saturating_sub(border_width)),
                style,
            )),
        );
        output.push(Line::from(Span::styled(
            " ".repeat(requested.saturating_sub(border_width)),
            style,
        )));
    }
    if let Some(border) = border {
        output = framed(output, requested, border, properties, style);
    }
    let top = dimension(
        properties,
        "marginTop",
        dimension(properties, "marginY", NO_SPACING),
    )
    .min(10);
    let bottom = dimension(
        properties,
        "marginBottom",
        dimension(properties, "marginY", NO_SPACING),
    )
    .min(10);
    for _ in 0..top {
        output.insert(0, Line::default());
    }
    output.extend((0..bottom).map(|_| Line::default()));
    output
}

fn row_lines(
    children: Vec<Vec<Line<'static>>>,
    gap: usize,
    width: usize,
    wrapping: bool,
) -> Vec<Line<'static>> {
    let mut output = Vec::new();
    let mut group: Vec<Line<'static>> = Vec::new();
    let mut used: usize = 0;
    for child in children {
        let child_width = child.iter().map(Line::width).max().unwrap_or(0);
        if wrapping && used > 0 && used + gap + child_width > width {
            output.append(&mut group);
            used = 0;
        }
        group.resize_with(group.len().max(child.len()), Line::default);
        for (index, line) in group.iter_mut().enumerate() {
            let padding = used.saturating_sub(line.width()) + if used > 0 { gap } else { 0 };
            line.spans.push(Span::raw(" ".repeat(padding)));
            if let Some(child) = child.get(index) {
                line.spans.extend(child.spans.clone());
            }
        }
        used += child_width + if used > 0 { gap } else { 0 };
    }
    output.append(&mut group);
    output
}

fn framed(
    mut lines: Vec<Line<'static>>,
    width: usize,
    border: &str,
    properties: &Map<String, Value>,
    style: Style,
) -> Vec<Line<'static>> {
    let (top_left, top_right, bottom_left, bottom_right, horizontal, vertical) = match border {
        "round" => ('╭', '╮', '╰', '╯', '─', '│'),
        "double" => ('╔', '╗', '╚', '╝', '═', '║'),
        _ => ('┌', '┐', '└', '┘', '─', '│'),
    };
    let border_style = properties
        .get("borderColor")
        .and_then(super::style::color)
        .map(|color| style.fg(color))
        .unwrap_or(style);
    for line in &mut lines {
        line.spans
            .insert(0, Span::styled(vertical.to_string(), border_style));
        line.spans
            .push(Span::styled(vertical.to_string(), border_style));
    }
    let edge = horizontal.to_string().repeat(width.saturating_sub(2));
    lines.insert(
        0,
        Line::from(Span::styled(
            format!("{top_left}{edge}{top_right}"),
            border_style,
        )),
    );
    lines.push(Line::from(Span::styled(
        format!("{bottom_left}{edge}{bottom_right}"),
        border_style,
    )));
    lines
}

fn dimension(properties: &Map<String, Value>, key: &str, default: usize) -> usize {
    properties
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}
