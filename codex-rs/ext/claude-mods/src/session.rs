use crate::Dispatch;
use crate::EventSink;
use crate::ModHost;
use crate::ModPlugin;
use crate::ModResult;
use crate::ModRuntime;
use crate::host::Host;
use codex_protocol::claude_mods::ModCommand;
use codex_protocol::claude_mods::ModPane;
use serde_json::Value;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

pub struct ModSession {
    pub(crate) reactive: Arc<crate::reactive::ReactiveState>,
    pub(crate) activation: tokio::sync::Semaphore,
    state: Mutex<State>,
    process_calls: Mutex<std::collections::HashSet<String>>,
    processes: tokio::sync::Mutex<std::collections::HashMap<u64, Value>>,
}

impl Default for ModSession {
    fn default() -> Self {
        Self {
            reactive: Arc::default(),
            activation: tokio::sync::Semaphore::new(1),
            state: Mutex::default(),
            process_calls: Mutex::default(),
            processes: tokio::sync::Mutex::default(),
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub signature: String,
    pub runtime: Option<ModRuntime>,
    pub host: Option<Arc<Host>>,
    pub failure: Option<String>,
    pub start_event: Value,
}

pub struct ModSessionRequest {
    pub plugins: Vec<ModPlugin>,
    pub codex_home: PathBuf,
    pub cwd: String,
    pub session_id: String,
    pub interactive: bool,
    pub delegate: Arc<dyn ModHost>,
    pub sink: EventSink,
}

impl ModSession {
    pub fn register_process_call(&self, id: String) {
        self.process_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(id);
    }
    pub fn owns_process_call(&self, id: &str) -> bool {
        self.process_calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(id)
    }
    pub async fn process_event(&self, id: u64) -> Option<Value> {
        self.processes.lock().await.get(&id).cloned()
    }
    pub async fn record_process(&self, id: u64, event: Value) {
        self.processes.lock().await.insert(id, event);
    }
    pub async fn forget_process(&self, id: u64) {
        self.processes.lock().await.remove(&id);
    }
    pub(crate) fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
    pub(crate) fn host(&self) -> Option<Arc<Host>> {
        self.state().host.clone()
    }
    pub fn check(&self) -> ModResult<()> {
        let state = self.state();
        if let Some(failure) = &state.failure {
            return Err(failure.clone());
        }
        if state
            .runtime
            .as_ref()
            .is_some_and(|runtime| !runtime.is_running())
        {
            return Err("mod runtime stopped".into());
        }
        Ok(())
    }
    pub async fn attach_ui(&self, interactive: bool) -> ModResult<()> {
        if !interactive {
            return self.check();
        }
        let _activation = self
            .activation
            .acquire()
            .await
            .map_err(|_| "mod activation stopped".to_string())?;
        if let Some(host) = self.host() {
            let activated = interactive
                && !host
                    .ui
                    .interactive
                    .load(std::sync::atomic::Ordering::Acquire);
            host.ui.attach(interactive).await;
            if activated {
                let mut event = self.state().start_event.clone();
                event["isInteractive"] = json!(true);
                self.observe("session.start", event).await?;
            }
        }
        Ok(())
    }

    pub async fn stop(&self) -> ModResult<()> {
        let _activation = self
            .activation
            .acquire()
            .await
            .map_err(|_| "mod activation stopped".to_string())?;
        let outcome = self.observe("session.end", json!({})).await;
        if let Some(runtime) = self.runtime() {
            runtime.shutdown();
        }
        *self.state() = State::default();
        self.reactive.clear();
        self.processes.lock().await.clear();
        outcome.map(|_| ())
    }

    pub fn runtime(&self) -> Option<ModRuntime> {
        self.state().runtime.clone()
    }

    pub async fn observe(&self, name: &str, input: Value) -> ModResult<Value> {
        self.check()?;
        let Some(runtime) = self.runtime() else {
            return Ok(input);
        };
        runtime
            .observe(Dispatch {
                name: name.into(),
                input,
            })
            .await
    }

    pub async fn commands(&self) -> Vec<ModCommand> {
        match self.host() {
            Some(host) => host.ui.commands().await,
            None => Vec::new(),
        }
    }

    pub async fn panes(&self) -> Vec<ModPane> {
        match self.host() {
            Some(host) => host.ui.panes().await,
            None => Vec::new(),
        }
    }

    pub async fn run_command(
        &self,
        plugin_id: &str,
        name: &str,
        arguments: &str,
    ) -> ModResult<String> {
        let registered = self
            .commands()
            .await
            .into_iter()
            .any(|command| command.plugin_id == plugin_id && command.name == name);
        if !registered {
            return Err(format!("unknown mod command {plugin_id}:{name}"));
        }
        let result = self
            .observe(
                "command.run",
                json!({"command":name,"args":arguments,"targetPluginId":plugin_id}),
            )
            .await?;
        Ok(result["text"].as_str().unwrap_or_default().to_string())
    }
}
