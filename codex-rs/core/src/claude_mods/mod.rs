mod compaction;
mod files;
mod hooks;
mod host;
pub(crate) mod lifecycle;
pub(crate) mod loader;
mod model;
mod model_options;
mod process;
mod prompt;
mod questions;
mod startup;
mod stream;
mod thread;
mod tool_events;
mod tools;

pub(crate) use hooks::ModHooks;

pub(crate) fn state(
    session: &crate::session::session::Session,
) -> std::sync::Arc<codex_claude_mods_extension::ModSession> {
    session
        .services
        .thread_extension_data
        .get_or_init(codex_claude_mods_extension::ModSession::default)
}

pub(crate) fn message(role: &str, text: String) -> codex_protocol::models::ResponseItem {
    codex_protocol::models::ResponseItem::Message {
        id: None,
        role: role.into(),
        content: vec![codex_protocol::models::ContentItem::InputText { text }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

mod commands;
mod patch_events;
mod tool_input;

#[cfg(test)]
mod compaction_tests;

mod shell_patch;

mod stream_chunk;
