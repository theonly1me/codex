use crate::history_cell::HistoryCell;
use codex_protocol::claude_mods::ModNode;
use ratatui::style::Style;
use ratatui::text::Line;

#[derive(Debug)]
pub(crate) struct ModHistoryCell {
    pub(crate) native: Box<dyn HistoryCell>,
    pub(crate) tree: ModNode,
}

impl HistoryCell for ModHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let ui = super::ModUi {
            native_lines: Some(self.native.display_lines(width)),
            ..Default::default()
        };
        super::tree::lines(&self.tree, &ui, "history", width, Style::default())
    }

    fn append_reasoning(&mut self, cell: Box<dyn HistoryCell>) -> Result<(), Box<dyn HistoryCell>> {
        self.native.append_reasoning(cell)
    }
    fn live_raw_lines(&self) -> Vec<Line<'static>> {
        self.native.live_raw_lines()
    }
    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.native.transcript_lines(width)
    }
    fn transcript_hyperlink_lines(
        &self,
        width: u16,
    ) -> Vec<crate::terminal_hyperlinks::HyperlinkLine> {
        self.native.transcript_hyperlink_lines(width)
    }
    fn expanded_hyperlink_lines(
        &self,
        width: u16,
    ) -> Vec<crate::terminal_hyperlinks::HyperlinkLine> {
        self.native.expanded_hyperlink_lines(width)
    }
    fn activity_disclosure(&self, width: u16) -> Option<crate::history_cell::ActivityDisclosure> {
        self.native.activity_disclosure(width)
    }
    fn has_stable_transcript_height(&self) -> bool {
        self.native.has_stable_transcript_height()
    }
    fn is_stream_continuation(&self) -> bool {
        self.native.is_stream_continuation()
    }
    fn transcript_animation_tick(&self) -> Option<u64> {
        self.native.transcript_animation_tick()
    }
    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.native.raw_lines()
    }
    fn activity_ids(&self) -> Vec<String> {
        self.native.activity_ids()
    }
}

pub(crate) fn component(cell: &dyn HistoryCell) -> Option<&'static str> {
    let cell = cell.as_any();
    if cell.is::<crate::history_cell::UserHistoryCell>() {
        Some("UserMessage")
    } else if cell.is::<crate::history_cell::AgentMarkdownCell>()
        || cell.is::<crate::history_cell::AgentMessageCell>()
    {
        Some("AssistantMessage")
    } else if cell.is::<crate::history_cell::FinalMessageSeparator>() {
        Some("TurnDuration")
    } else if cell
        .downcast_ref::<crate::exec_cell::ExecCell>()
        .is_some_and(|cell| cell.iter_calls().count() > 1)
    {
        Some("ToolGroup")
    } else if cell.is::<crate::exec_cell::ExecCell>()
        || cell.is::<crate::history_cell::PatchHistoryCell>()
        || cell.is::<crate::history_cell::McpToolCallCell>()
    {
        Some("ToolUse")
    } else {
        None
    }
}
