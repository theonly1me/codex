use super::*;
use codex_protocol::claude_mods::ClaudeModEvent;
use codex_protocol::claude_mods::ModNode;
use std::str::FromStr;

impl ChatWidget {
    pub(crate) fn claude_mod_event(&mut self, event: ClaudeModEvent) {
        let hint = None;
        match event {
            ClaudeModEvent::Copy { text } => {
                self.app_event_tx.send(AppEvent::ClaudeModCopy { text })
            }
            ClaudeModEvent::ThemePicker => {
                self.dispatch_command(crate::slash_command::SlashCommand::Theme)
            }
            ClaudeModEvent::Command { command } => {
                if crate::slash_command::SlashCommand::from_str(&command.name).is_ok()
                    && !self.mod_ui.commands.iter().any(|existing| {
                        existing.plugin_id == command.plugin_id && existing.name == command.name
                    })
                {
                    self.add_info_message(
                        format!(
                            "Command /{} is built in. Use /{}.",
                            command.name, command.qualified_name
                        ),
                        hint,
                    );
                }
                self.mod_ui.commands.retain(|existing| {
                    existing.plugin_id != command.plugin_id || existing.name != command.name
                });
                self.mod_ui.commands.push(command);
                self.set_mod_command_names();
            }
            ClaudeModEvent::Open { pane } => {
                self.mod_ui.panes.retain(|existing| {
                    existing.plugin_id != pane.plugin_id || existing.id != pane.id
                });
                if pane.focus {
                    self.mod_ui.active_pane = Some(crate::claude_mods::pane_request_id(&pane));
                    self.mod_ui.visible_pane = self.mod_ui.active_pane.clone();
                }
                self.mod_ui.panes.push(pane);
                self.refresh_mod_ui();
            }
            ClaudeModEvent::Close { plugin_id, id } => {
                self.mod_ui
                    .panes
                    .retain(|pane| pane.plugin_id != plugin_id || pane.id != id);
                let request_id = format!("pane:{plugin_id}:{id}");
                self.mod_ui.trees.remove(&request_id);
                if self.mod_ui.visible_pane.as_ref() == Some(&request_id) {
                    self.mod_ui.visible_pane = None;
                }
                if self.mod_ui.active_pane.as_ref() == Some(&request_id) {
                    self.mod_ui.active_pane = None;
                }
                self.refresh_mod_ui();
            }
            ClaudeModEvent::Invalidate { .. } | ClaudeModEvent::Config { .. } => {
                self.refresh_mod_ui()
            }
            ClaudeModEvent::Raster {
                request_id,
                key,
                cells,
                ..
            } => {
                self.mod_ui.rasters.insert((request_id, key), cells);
            }
            ClaudeModEvent::Status { plugin_id, text } => {
                self.mod_ui.status.remove(&plugin_id);
                if let Some(text) = text {
                    self.mod_ui.status.insert(plugin_id, text);
                }
            }
            ClaudeModEvent::Toast { text, .. } => self.add_info_message(text, hint),
            ClaudeModEvent::Error { plugin_id, message } => {
                self.add_error_message(format!("Mod {plugin_id}: {message}"))
            }
            ClaudeModEvent::PromptFill { text, .. } => {
                self.bottom_pane
                    .set_composer_text(text, Vec::new(), Vec::new())
            }
        }
        self.request_redraw();
    }

    pub(crate) fn refresh_mod_ui(&mut self) {
        if self.mod_ui.pending {
            self.mod_ui.dirty = true;
            return;
        }
        self.mod_ui.pending = true;
        self.app_event_tx.send(AppEvent::ClaudeModRefresh);
    }

    pub(crate) fn set_mod_command_names(&mut self) {
        let names = self
            .mod_ui
            .commands
            .iter()
            .flat_map(|command| [command.name.clone(), command.qualified_name.clone()])
            .collect();
        self.bottom_pane.set_mod_command_names(names);
    }

    pub(super) fn dispatch_mod_command(&mut self, text: &str) -> bool {
        let Some(command_line) = text.strip_prefix('/') else {
            return false;
        };
        let (name, arguments) = command_line
            .split_once(char::is_whitespace)
            .unwrap_or((command_line, ""));
        if crate::slash_command::SlashCommand::from_str(name).is_ok() {
            return false;
        }
        let Some(command) = self.mod_ui.command(name) else {
            let alternatives = self
                .mod_ui
                .commands
                .iter()
                .filter(|command| command.name == name)
                .map(|command| format!("/{}", command.qualified_name))
                .collect::<Vec<_>>();
            if alternatives.len() > 1 {
                self.add_error_message(format!(
                    "Command /{name} is shared. Use {}",
                    alternatives.join(" or ")
                ));
                return true;
            }
            return false;
        };
        self.app_event_tx.send(AppEvent::ClaudeModCommand {
            plugin_id: command.plugin_id.clone(),
            name: command.name.clone(),
            arguments: arguments.trim().into(),
        });
        true
    }

    pub(super) fn mod_key(&mut self, key: KeyEvent) -> bool {
        if self.bottom_pane.has_active_view() {
            return false;
        }
        let Some(interaction) = self.mod_ui.key(key) else {
            return false;
        };
        if !matches!(&interaction, codex_app_server_protocol::ClaudeModInteraction::Callback { callback_id, .. } if callback_id.is_empty())
        {
            self.app_event_tx
                .send(AppEvent::ClaudeModControl { interaction });
        }
        self.refresh_mod_ui();
        self.request_redraw();
        true
    }

    pub(crate) fn mod_frames(&mut self, trees: Vec<(String, ModNode)>) {
        self.mod_ui.pending = false;
        self.mod_ui.trees = trees.into_iter().collect();
        let header = self
            .mod_ui
            .trees
            .get("Spinner")
            .map(crate::claude_mods::text);
        self.bottom_pane.set_mod_header(header);
        self.mod_ui.choose_initial_focus();
        if self.mod_ui.dirty {
            self.mod_ui.dirty = false;
            self.refresh_mod_ui();
        }
        self.request_redraw();
    }

    pub(crate) fn mod_spinner_word(&self) -> String {
        self.bottom_pane
            .status_widget()
            .map(|status| status.header().to_string())
            .unwrap_or_else(|| "Working".into())
    }
    pub(crate) fn mod_has_survey(&self) -> bool {
        self.bottom_pane.has_active_view()
    }
    pub(crate) fn mod_is_working(&self) -> bool {
        self.bottom_pane.is_task_running()
    }
}
