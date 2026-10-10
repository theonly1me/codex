use super::ModUi;
use super::control_tree::collect;
use super::controls::local_consumed;
use codex_app_server_protocol::ClaudeModInteraction;

impl ModUi {
    pub(crate) fn choose_initial_focus(&mut self) {
        if self.visible_pane.as_ref().is_none_or(|visible| {
            !self
                .panes
                .iter()
                .any(|pane| super::pane_request_id(pane) == *visible)
        }) {
            self.visible_pane = self.panes.first().map(super::pane_request_id);
        }
        if self.focused_pane == self.active_pane {
            return;
        }
        self.focused_pane = self.active_pane.clone();
        self.focus = 0;
        let controls = self.controls();
        if let Some(index) = controls.iter().position(|control| control.auto_focus) {
            self.focus = index;
        }
    }

    pub(super) fn focus_next(&mut self) -> Option<ClaudeModInteraction> {
        let mut targets = self
            .panes
            .iter()
            .map(super::pane_request_id)
            .collect::<Vec<_>>();
        let mut band_controls = Vec::new();
        if let Some(band) = self.trees.get("AbovePrompt") {
            collect(band, &mut band_controls);
        }
        if !band_controls.is_empty() {
            targets.push("AbovePrompt".into());
        }
        if targets.is_empty() {
            return None;
        }
        let selected = self
            .active_pane
            .as_ref()
            .or(self.visible_pane.as_ref())
            .and_then(|active| targets.iter().position(|target| target == active))
            .unwrap_or(0);
        let index = if self.active_pane.is_some() {
            (selected + 1) % targets.len()
        } else {
            selected
        };
        let target = targets[index].clone();
        if target != "AbovePrompt" {
            self.visible_pane = Some(target.clone());
        }
        self.active_pane = Some(target.clone());
        self.choose_initial_focus();
        if let Some(pane) = self
            .panes
            .iter()
            .find(|pane| super::pane_request_id(pane) == target)
        {
            Some(ClaudeModInteraction::Focus {
                plugin_id: pane.plugin_id.clone(),
                id: pane.id.clone(),
                is_focused: true,
                element: None,
            })
        } else {
            Some(local_consumed())
        }
    }
}
