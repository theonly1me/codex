use crate::app::App;
use crate::app::owned_transcript::tests::attach_thread;
use crate::app::owned_transcript::tests::buffer_text;
use crate::render::renderable::Renderable;
use base64::Engine;
use codex_protocol::ThreadId;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModPane;
use color_eyre::eyre::Result;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::style::Color;
use serde_json::json;

fn cells(foreground: u32) -> String {
    let words = [u32::from('▀'), foreground, 0x01000000];
    base64::engine::general_purpose::STANDARD.encode(
        words
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
}

async fn app_with_scene() -> Result<App> {
    let mut app = crate::app::test_support::make_test_app().await;
    attach_thread(&mut app, ThreadId::new());
    app.chat_widget
        .apply_external_edit("preserved draft".into());
    app.chat_widget.claude_mod_event(ClaudeModEvent::Open {
        pane: ModPane {
            plugin_id: "samurai-dojo@claude-code-mods".into(),
            id: "samurai-dojo".into(),
            title: "Samurai Dojo".into(),
            rows: 2,
            columns: 62,
            is_placed: true,
            focus: false,
            close_on_escape: true,
        },
    });
    let foreground = 0xff0000;
    let scene: ModNode = serde_json::from_value(json!({
        "element":"Raster",
        "props":{"key":"samurai-dojo:stage","columns":1,"rows":1,"cells":cells(foreground)}
    }))?;
    app.chat_widget.mod_frames(vec![(
        "pane:samurai-dojo@claude-code-mods:samurai-dojo".into(),
        scene,
    )]);
    Ok(app)
}

fn assert_scene(buffer: &Buffer, foreground: Color) {
    let text = buffer_text(buffer);
    assert_eq!(text.matches("Samurai Dojo").count(), 1, "{text}");
    assert!(text.contains("preserved draft"), "{text}");
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.symbol() == "▀" && cell.fg == foreground),
        "The mod scene is absent from the visible terminal: {text}"
    );
}

#[tokio::test]
async fn owned_transcript_displays_mod_animation_and_preserves_draft_cursor() -> Result<()> {
    let mut app = app_with_scene().await?;
    let mut tui = crate::tui::test_support::make_test_tui()?;
    let owned = true;
    tui.set_owned_screen(owned)?;
    let size = Size::new(80, 24);
    app.render_owned_transcript(&mut tui, size)?;
    let before = crate::custom_terminal::test_support::last_rendered_buffer(&tui.terminal);
    assert_scene(before, "#ff0000".parse()?);
    let cursor = tui.terminal.last_known_cursor_pos;
    let foreground = 0x00ff00;
    app.chat_widget.claude_mod_event(ClaudeModEvent::Raster {
        plugin_id: "samurai-dojo@claude-code-mods".into(),
        request_id: "samurai-dojo".into(),
        key: "samurai-dojo:stage".into(),
        cells: cells(foreground),
    });
    app.render_owned_transcript(&mut tui, size)?;
    assert_scene(
        crate::custom_terminal::test_support::last_rendered_buffer(&tui.terminal),
        "#00ff00".parse()?,
    );
    assert_eq!(tui.terminal.last_known_cursor_pos, cursor);
    let owned = false;
    tui.set_owned_screen(owned)?;
    Ok(())
}

#[tokio::test]
async fn inline_transcript_displays_one_mod_scene_and_preserves_composer_space() -> Result<()> {
    let app = app_with_scene().await?;
    let renderable = app.chat_widget.as_renderable();
    let width = 100;
    let area = Rect::new(0, 0, width, renderable.desired_height(width));
    let mut buffer = Buffer::empty(area);
    renderable.render(area, &mut buffer);
    assert_scene(&buffer, "#ff0000".parse()?);
    assert!(renderable.cursor_pos(area).is_some());
    let small = Rect::new(0, 0, 100, 5);
    let mut buffer = Buffer::empty(small);
    renderable.render(small, &mut buffer);
    assert!(buffer_text(&buffer).contains("preserved draft"));
    assert!(renderable.cursor_pos(small).is_some());
    Ok(())
}
