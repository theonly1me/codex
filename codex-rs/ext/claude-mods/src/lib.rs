mod activation;
mod host;
mod loader;
mod loading;
mod render;
mod session;
mod storage;
mod ui;
mod ui_dimensions;

pub use loader::InstalledModLoader;
pub use loader::LoadedMods;
pub use loader::ModLoader;
pub use loader::ModSource;
pub use loading::ModSessionContext;
pub use session::ModSession;
pub use session::ModSessionRequest;

pub use codex_mod_runtime::Dispatch;
pub use codex_mod_runtime::HostCall;
pub use codex_mod_runtime::ModHost;
pub use codex_mod_runtime::ModPlugin;
pub use codex_mod_runtime::ModResult;
pub use codex_mod_runtime::ModRuntime;

pub type EventSink = std::sync::Arc<
    dyn Fn(codex_protocol::claude_mods::ClaudeModEvent) -> futures::future::BoxFuture<'static, ()>
        + Send
        + Sync,
>;

mod reload;
mod store;
mod ui_commands;

mod controls;
mod reactive;
