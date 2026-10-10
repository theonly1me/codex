mod composition;
mod controls;
mod focus;
pub(crate) mod history;
mod raster;
mod style;
mod tree;

use crate::render::renderable::Renderable;
use codex_protocol::claude_mods::ModCommand;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModPane;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct ModUi {
    pub(crate) commands: Vec<ModCommand>,
    pub(crate) panes: Vec<ModPane>,
    pub(crate) trees: BTreeMap<String, ModNode>,
    pub(crate) rasters: BTreeMap<(String, String), String>,
    pub(crate) input: String,
    pub(crate) focus: usize,
    pub(crate) pending: bool,
    pub(crate) dirty: bool,
    pub(crate) active_pane: Option<String>,
    pub(crate) visible_pane: Option<String>,
    pub(crate) focused_pane: Option<String>,
    pub(crate) status: BTreeMap<String, String>,
    pub(crate) native_lines: Option<Vec<Line<'static>>>,
}

impl ModUi {
    pub(crate) fn command(&self, name: &str) -> Option<&ModCommand> {
        let qualified = self
            .commands
            .iter()
            .find(|command| command.qualified_name == name);
        qualified.or_else(|| {
            let mut candidates = self.commands.iter().filter(|command| command.name == name);
            let command = candidates.next()?;
            candidates.next().is_none().then_some(command)
        })
    }

    pub(crate) fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let visible = self
            .active_pane
            .clone()
            .filter(|active| active != "AbovePrompt")
            .or_else(|| self.visible_pane.clone())
            .or_else(|| self.panes.first().map(pane_request_id));
        for (request_id, tree) in &self.trees {
            if request_id != "AbovePrompt"
                && request_id != "PromptHint"
                && !request_id.starts_with("pane:")
            {
                continue;
            }
            if request_id.starts_with("pane:") && visible.as_ref() != Some(request_id) {
                continue;
            }
            let render_id = self
                .panes
                .iter()
                .find(|pane| pane_request_id(pane) == *request_id)
                .map(|pane| pane.id.as_str())
                .unwrap_or(request_id);
            let rendered = tree::lines(tree, self, render_id, width, Style::default());
            if request_id.starts_with("pane:") {
                let rows = self
                    .panes
                    .iter()
                    .find(|pane| pane_request_id(pane) == *request_id)
                    .map(|pane| pane.rows)
                    .unwrap_or(20);
                if let Some(pane) = self
                    .panes
                    .iter()
                    .find(|pane| pane_request_id(pane) == *request_id)
                {
                    lines.push(Line::from(format!("{}  [F6: next pane]", pane.title)));
                }
                let available = crossterm::terminal::size()
                    .map(|(_, rows)| rows.saturating_sub(10).max(1))
                    .unwrap_or(20);
                lines.extend(rendered.into_iter().take(usize::from(rows.min(available))));
            } else {
                lines.extend(rendered.into_iter().take(10));
            }
        }
        for text in self.status.values() {
            lines.push(Line::from(text.clone()));
        }
        lines
    }
}

pub(crate) fn pane_request_id(pane: &ModPane) -> String {
    format!("pane:{}:{}", pane.plugin_id, pane.id)
}

impl Renderable for ModUi {
    fn render(&self, area: Rect, buffer: &mut Buffer) {
        Paragraph::new(self.lines(area.width))
            .wrap(Wrap { trim: false })
            .render(area, buffer);
    }

    fn desired_height(&self, width: u16) -> u16 {
        u16::try_from(self.lines(width).len())
            .unwrap_or(u16::MAX)
            .min(80)
    }
}

pub(crate) fn text(node: &ModNode) -> String {
    match node {
        ModNode::Text(text) => text.clone(),
        ModNode::Number(number) => number.to_string(),
        ModNode::Children(children) => children.iter().map(text).collect(),
        ModNode::Element(element) => element.children.iter().map(text).collect(),
        ModNode::Empty => String::new(),
    }
}

mod control_tree;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod status_tests;

#[cfg(test)]
mod band_tests;

mod layout;
