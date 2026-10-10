use super::ModUi;
use codex_app_server_protocol::ClaudeModInteraction;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use serde_json::json;

#[test]
fn a_prompt_band_accepts_answers_without_an_open_pane() -> Result<(), Box<dyn std::error::Error>> {
    let tree = serde_json::from_value(json!({"element":"Box","children":[
        {"element":"Button","props":{"label":"One","hotkey":"1","onPress":{"callbackId":"one"}}},
        {"element":"Input","props":{"key":"answer","onSubmit":{"callbackId":"answer"}}}
    ]}))?;
    let mut ui = ModUi::default();
    ui.trees.insert("AbovePrompt".into(), tree);
    assert!(
        ui.key(KeyEvent::new(KeyCode::F(6), KeyModifiers::NONE))
            .is_some()
    );
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(ClaudeModInteraction::Callback {
            callback_id: "one".into(),
            value: serde_json::Value::Null
        })
    );
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
        Some(ClaudeModInteraction::Callback {
            callback_id: "one".into(),
            value: serde_json::Value::Null
        })
    );
    assert!(
        ui.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
            .is_some()
    );
    assert!(
        ui.key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
            .is_some()
    );
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(ClaudeModInteraction::Callback {
            callback_id: "answer".into(),
            value: json!("x")
        })
    );
    assert!(
        ui.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .is_some()
    );
    assert_eq!(
        ui.key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE)),
        None
    );
    Ok(())
}
