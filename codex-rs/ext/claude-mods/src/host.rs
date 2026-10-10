use crate::EventSink;
use crate::HostCall;
use crate::ModHost;
use crate::ModPlugin;
use crate::ModResult;
use crate::storage::Storage;
use crate::ui::Ui;
use codex_protocol::claude_mods::ClaudeModEvent;
use futures::future::BoxFuture;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

pub(crate) struct Host {
    pub reactive: Arc<crate::reactive::ReactiveState>,
    pub runtime: tokio::sync::OnceCell<codex_claude_mod_runtime::WeakModRuntime>,
    pub plugins: HashMap<String, ModPlugin>,
    pub storage: Storage,
    pub ui: Ui,
    pub delegate: Arc<dyn ModHost>,
    pub sink: EventSink,
}

impl ModHost for Host {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move {
            if !self.plugins.contains_key(&call.plugin) {
                return Err(format!("unknown mod {}", call.plugin));
            }
            let arguments = &call.arguments;
            let first = arguments.first().unwrap_or(&Value::Null);
            match call.method.as_str() {
                "clock.now" => Ok(serde_json::json!(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|error| error.to_string())?
                        .as_millis()
                )),
                "env.get" if first.as_str() == Some("HOME") => {
                    Ok(serde_json::json!(self.storage.home))
                }
                "store.get" => {
                    self.storage
                        .get(&call.plugin, "store", string(first)?)
                        .await
                }
                "store.set" => {
                    self.storage
                        .set(
                            &call.plugin,
                            "store",
                            string(first)?,
                            arguments.get(1).cloned().unwrap_or(Value::Null),
                        )
                        .await
                }
                "store.delete" => self.storage.delete(&call.plugin, string(first)?).await,
                "state.get" | "state.set" => {
                    let (value, changed) = self.reactive.call(&call, &self.plugins)?;
                    if changed {
                        (self.sink)(ClaudeModEvent::Invalidate {
                            plugin_id: call.plugin,
                        })
                        .await;
                    }
                    Ok(value)
                }
                "fs.read"
                    if first.as_str().is_some_and(|path| {
                        Path::new(path).starts_with(&self.plugins[&call.plugin].root)
                    }) =>
                {
                    let plugin = &self.plugins[&call.plugin];
                    let path = Path::new(string(first)?)
                        .canonicalize()
                        .map_err(|error| error.to_string())?;
                    if !path.starts_with(&plugin.root) {
                        return Err("mod asset escapes plugin root".into());
                    }
                    if std::fs::metadata(&path)
                        .map_err(|error| error.to_string())?
                        .len()
                        > 2 * 1024 * 1024
                    {
                        return Err("mod asset exceeds 2 MiB".into());
                    }
                    std::fs::read_to_string(path)
                        .map(Value::String)
                        .map_err(|error| error.to_string())
                }
                "fs.read"
                    if first
                        .as_str()
                        .is_some_and(|path| self.storage.owns_path(Path::new(path))) =>
                {
                    self.storage
                        .read_owned(Path::new(string(first)?))
                        .await
                        .map(Value::String)
                }
                "fs.write"
                    if first
                        .as_str()
                        .is_some_and(|path| self.storage.owns_path(Path::new(path))) =>
                {
                    self.storage
                        .write_owned(
                            Path::new(string(first)?),
                            string(arguments.get(1).unwrap_or(&Value::Null))?.into(),
                        )
                        .await
                }
                "ui.copy" => {
                    let text = string(&first["text"])?;
                    if text.len() > 100000 {
                        return Err("mod clipboard text exceeds 100000 bytes".into());
                    }
                    (self.sink)(ClaudeModEvent::Copy { text: text.into() }).await;
                    Ok(Value::Null)
                }
                "ui.ask" | "ui.focus" | "ui.scroll" | "command.run" => {
                    self.delegate.call(call).await
                }
                "ui.close" => {
                    let result = self.ui.call(&call, &self.sink).await?;
                    if let Some(runtime) = self
                        .runtime
                        .get()
                        .and_then(codex_claude_mod_runtime::WeakModRuntime::upgrade)
                    {
                        let origin = first
                            .get("origin")
                            .cloned()
                            .unwrap_or_else(|| serde_json::json!({"kind":"engine"}));
                        runtime.observe(crate::Dispatch { name: "ui.close".into(), input: serde_json::json!({"id":first["id"],"origin":origin,"targetPluginId":call.plugin}) }).await?;
                    }
                    Ok(result)
                }
                "config.set"
                    if self
                        .runtime
                        .get()
                        .and_then(codex_claude_mod_runtime::WeakModRuntime::upgrade)
                        .is_some() =>
                {
                    let runtime = self
                        .runtime
                        .get()
                        .and_then(codex_claude_mod_runtime::WeakModRuntime::upgrade)
                        .ok_or("mod runtime stopped")?;
                    let key = first
                        .as_str()
                        .map(Value::from)
                        .unwrap_or_else(|| first["key"].clone());
                    let value = first
                        .get("value")
                        .or_else(|| call.arguments.get(1))
                        .cloned()
                        .unwrap_or(Value::Null);
                    runtime.dispatch(crate::Dispatch { name: "config.set".into(), input: serde_json::json!({"key":key,"value":value,"origin":{"kind":"plugin","plugin":call.plugin}}) }, |input| {
                        let changed = HostCall { plugin: call.plugin.clone(), method: "config.set".into(), arguments: vec![input] };
                        async move { self.ui.call(&changed, &self.sink).await }
                    }).await
                }
                method
                    if method.starts_with("ui.")
                        || method.starts_with("command.")
                        || method.starts_with("config.") =>
                {
                    self.ui.call(&call, &self.sink).await
                }
                "runtime.error" => {
                    (self.sink)(ClaudeModEvent::Error {
                        plugin_id: call.plugin,
                        message: string(first)?.into(),
                    })
                    .await;
                    Ok(Value::Null)
                }
                _ => self.delegate.call(call).await,
            }
        })
    }
}

pub(crate) fn string(value: &Value) -> ModResult<&str> {
    value
        .as_str()
        .ok_or_else(|| "mod API argument must be a string".to_string())
}
