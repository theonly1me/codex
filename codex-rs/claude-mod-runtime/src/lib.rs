mod actor;
mod bridge;
mod engine;
mod heap;
mod hook_sources;
mod module_loader;
mod options;
mod service;
mod source;
mod source_signature;
mod transform;
mod value;
mod watchdog;

pub use bridge::HostCall;
pub use bridge::ModHost;
pub use options::resolve_options;
pub use service::Dispatch;
pub use service::ModRuntime;
pub use service::WeakModRuntime;
pub use source::ModHookSource;
pub use source::ModPlugin;
pub use source::ModPluginRequest;

pub type ModResult<Value> = Result<Value, String>;
