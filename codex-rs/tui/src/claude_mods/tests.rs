use super::ModUi;
use super::pane_request_id;
use crate::render::renderable::Renderable;
use base64::Engine;
use codex_app_server_protocol::ClaudeModInteraction;
use codex_protocol::claude_mods::ModNode;
use codex_protocol::claude_mods::ModPane;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use serde_json::json;

fn pane(plugin: &str, id: &str) -> ModPane {
    ModPane {
        plugin_id: plugin.into(),
        id: id.into(),
        title: id.into(),
        rows: 10,
        columns: 40,
        is_placed: true,
        focus: true,
        close_on_escape: true,
    }
}

#[test]
fn raster_frames_reach_the_terminal_with_their_colors() -> Result<(), Box<dyn std::error::Error>> {
    let pane = pane("game", "game-pane");
    let id = pane_request_id(&pane);
    let bytes = [
        u32::from('A'),
        0xff0000,
        0x01000000,
        u32::from('B'),
        0x00ff00,
        0x01000000,
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect::<Vec<_>>();
    let cells = base64::engine::general_purpose::STANDARD.encode(bytes);
    let tree: ModNode = serde_json::from_value(
        json!({"element":"Raster","props":{"key":"scene","columns":2,"rows":1,"cells":cells}}),
    )?;
    let mut ui = ModUi {
        panes: vec![pane],
        active_pane: Some(id.clone()),
        ..Default::default()
    };
    ui.trees.insert(id, tree);
    let area = Rect::new(0, 0, 40, 5);
    let mut buffer = Buffer::empty(area);
    ui.render(area, &mut buffer);
    assert_eq!(buffer[(0, 1)].symbol(), "A");
    assert_eq!(buffer[(1, 1)].symbol(), "B");
    assert_eq!(buffer[(0, 1)].fg, "#ff0000".parse::<Color>()?);
    assert_eq!(buffer[(1, 1)].fg, "#00ff00".parse::<Color>()?);
    Ok(())
}

#[test]
fn pane_focus_stays_on_the_selected_control_after_new_frames()
-> Result<(), Box<dyn std::error::Error>> {
    let first = pane("first", "first-pane");
    let second = pane("second", "second-pane");
    let first_id = pane_request_id(&first);
    let tree = serde_json::from_value(json!({"element":"Box","children":[
        {"element":"Input","props":{"key":"answer","autoFocus":true,"onSubmit":{"callbackId":"answer"}}},
        {"element":"Button","props":{"key":"keep","label":"Keep","onPress":{"callbackId":"keep"}}}
    ]}))?;
    let mut ui = ModUi {
        panes: vec![first, second],
        active_pane: Some(first_id.clone()),
        ..Default::default()
    };
    ui.trees.insert(first_id, tree);
    ui.choose_initial_focus();
    let selected = ui.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    assert!(
        matches!(selected, Some(ClaudeModInteraction::Focus { plugin_id, id, element, .. }) if plugin_id == "first" && id == "first-pane" && element.as_deref() == Some("keep"))
    );
    ui.choose_initial_focus();
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(ClaudeModInteraction::Callback {
            callback_id: "keep".into(),
            value: serde_json::Value::Null
        })
    );
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Some(ClaudeModInteraction::Close {
            plugin_id: "first".into(),
            id: "first-pane".into()
        })
    );
    Ok(())
}

#[test]
fn an_unfocused_pane_keeps_typing_in_the_composer() -> Result<(), Box<dyn std::error::Error>> {
    let mut pane = pane("game", "game-pane");
    pane.focus = false;
    let id = pane_request_id(&pane);
    let tree = serde_json::from_value(
        json!({"element":"Button","props":{"label":"Keep","hotkey":"k","onPress":{"callbackId":"keep"}}}),
    )?;
    let mut ui = ModUi {
        panes: vec![pane],
        ..Default::default()
    };
    ui.trees.insert(id.clone(), tree);
    ui.choose_initial_focus();
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
        None
    );
    assert!(
        ui.key(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE))
            .is_some()
    );
    assert_eq!(ui.active_pane.as_deref(), Some(id.as_str()));
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
        Some(ClaudeModInteraction::Callback {
            callback_id: "keep".into(),
            value: serde_json::Value::Null
        })
    );
    Ok(())
}
