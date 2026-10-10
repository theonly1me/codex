use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;
use crate::render::renderable::Renderable;
use crate::status_indicator_widget::StatusIndicatorWidget;
use crate::status_indicator_widget::StatusTimer;
use codex_protocol::claude_mods::ModNode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use tokio::sync::mpsc::unbounded_channel;

#[test]
fn an_empty_mod_frame_preserves_the_native_working_indicator() {
    let (sender, _receiver) = unbounded_channel::<AppEvent>();
    let animations_enabled = false;
    let mut status = StatusIndicatorWidget::new(
        AppEventSender::new(sender),
        crate::tui::FrameRequester::test_dummy(),
        animations_enabled,
        Default::default(),
    );
    status.set_mod_header(Some(super::text(&ModNode::Empty)));
    let timer = StatusTimer::default();
    let area = Rect::new(0, 0, 40, 2);
    let mut buffer = Buffer::empty(area);
    status.with_timer(&timer).render(area, &mut buffer);
    let text = buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();
    assert!(text.contains("Working"), "{text}");
}
