mod files;
mod host;
pub(crate) mod lifecycle;
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

pub(crate) use startup::start;
pub(crate) use tools::dispatch;

pub(crate) fn message(role: &str, text: String) -> codex_protocol::models::ResponseItem {
    codex_protocol::models::ResponseItem::Message {
        id: None,
        role: role.into(),
        content: vec![codex_protocol::models::ContentItem::InputText { text }],
        phase: None,
        internal_chat_message_metadata_passthrough: None,
    }
}

pub(crate) use lifecycle::compose;
pub(crate) use lifecycle::turn_complete;
pub(crate) use lifecycle::turn_start;
pub(crate) use prompt::submit;
pub(crate) use stream::wrap as wrap_stream;

mod commands;
mod patch_events;
mod tool_input;

mod shell_patch;

mod stream_chunk;
