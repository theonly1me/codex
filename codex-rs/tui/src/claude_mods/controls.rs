use super::ModUi;
use super::control_tree::Control;
use super::control_tree::collect;
use codex_app_server_protocol::ClaudeModInteraction;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use serde_json::Value;
use serde_json::json;

impl ModUi {
    pub(super) fn controls(&self) -> Vec<Control> {
        let mut controls = Vec::new();
        if let Some(tree) = self.active_pane.as_ref().and_then(|id| self.trees.get(id)) {
            collect(tree, &mut controls);
        }
        controls
    }

    pub(super) fn focused_callback(&self) -> Option<String> {
        let controls = self.controls();
        controls
            .get(self.focus % controls.len().max(1))
            .map(|control| control.callback.clone())
    }

    pub(crate) fn key(&mut self, key: KeyEvent) -> Option<ClaudeModInteraction> {
        if key.kind == KeyEventKind::Release
            || key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        if key.code == KeyCode::F(6) {
            return self.focus_next();
        }
        if key.code == KeyCode::Esc
            && let Some(active) = self.active_pane.take()
        {
            if let Some(pane) = self
                .panes
                .iter()
                .find(|pane| super::pane_request_id(pane) == active)
                && pane.close_on_escape
            {
                return Some(ClaudeModInteraction::Close {
                    plugin_id: pane.plugin_id.clone(),
                    id: pane.id.clone(),
                });
            }
            return Some(local_consumed());
        }
        let controls = self.controls();
        if controls.is_empty() {
            return None;
        }
        if key.code == KeyCode::Tab {
            self.focus = (self.focus + 1) % controls.len();
            if self.active_pane.as_deref() == Some("AbovePrompt") {
                return Some(local_consumed());
            }
            return Some(ClaudeModInteraction::Focus {
                plugin_id: self
                    .panes
                    .iter()
                    .find(|pane| self.active_pane.as_ref() == Some(&super::pane_request_id(pane)))
                    .map(|pane| pane.plugin_id.clone())
                    .unwrap_or_default(),
                id: self
                    .panes
                    .iter()
                    .find(|pane| self.active_pane.as_ref() == Some(&super::pane_request_id(pane)))
                    .map(|pane| pane.id.clone())
                    .unwrap_or_default(),
                is_focused: true,
                element: controls
                    .get(self.focus)
                    .map(|control| control.element.clone()),
            });
        }
        let focused = controls.get(self.focus % controls.len())?;
        if focused.input {
            match key.code {
                KeyCode::Char(character) => {
                    self.input.push(character);
                    return Some(local_consumed());
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    return Some(local_consumed());
                }
                KeyCode::Enter => {
                    return Some(ClaudeModInteraction::Callback {
                        callback_id: focused.callback.clone(),
                        value: json!(std::mem::take(&mut self.input)),
                    });
                }
                _ => {}
            }
        }
        if key.code == KeyCode::Enter {
            return Some(ClaudeModInteraction::Callback {
                callback_id: focused.callback.clone(),
                value: Value::Null,
            });
        }
        let name = match key.code {
            KeyCode::Char(' ') => "space".into(),
            KeyCode::Char(character) => character.to_string(),
            KeyCode::Left => "left".into(),
            KeyCode::Right => "right".into(),
            KeyCode::Up => "up".into(),
            KeyCode::Down => "down".into(),
            _ => return None,
        };
        let control = controls
            .iter()
            .find(|control| control.hotkey.eq_ignore_ascii_case(&name))?;
        Some(ClaudeModInteraction::Callback {
            callback_id: control.callback.clone(),
            value: Value::Null,
        })
    }
}

pub(super) fn local_consumed() -> ClaudeModInteraction {
    ClaudeModInteraction::Callback {
        callback_id: String::new(),
        value: Value::Null,
    }
}
