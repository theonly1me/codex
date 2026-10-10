use crate::ModHost;
use crate::ModPlugin;
use crate::ModResult;
use crate::actor::Command;
use crate::bridge::PendingDispatch;
use serde_json::Value;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use tokio::sync::oneshot;

pub struct Dispatch {
    pub name: String,
    pub input: Value,
}

#[derive(Clone)]
pub struct ModRuntime {
    inner: Arc<RuntimeHandle>,
}

#[derive(Clone)]
pub struct WeakModRuntime {
    inner: std::sync::Weak<RuntimeHandle>,
}

impl WeakModRuntime {
    pub fn upgrade(&self) -> Option<ModRuntime> {
        self.inner.upgrade().map(|inner| ModRuntime { inner })
    }
}

struct RuntimeHandle {
    sender: mpsc::Sender<Command>,
    sequence: AtomicU64,
    isolate: v8::IsolateHandle,
    stopping: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let _ = self.sender.send(Command::Shutdown);
        self.isolate.terminate_execution();
    }
}

struct DispatchGuard {
    id: u64,
    sender: mpsc::Sender<Command>,
}

impl Drop for DispatchGuard {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Cancel { id: self.id });
    }
}

impl ModRuntime {
    pub fn is_running(&self) -> bool {
        !self.inner.stopping.load(Ordering::Acquire)
    }
    pub fn shutdown(&self) {
        self.inner.stopping.store(true, Ordering::Release);
        let _ = self.inner.sender.send(Command::Shutdown);
        self.inner.isolate.terminate_execution();
    }
    pub fn downgrade(&self) -> WeakModRuntime {
        WeakModRuntime {
            inner: Arc::downgrade(&self.inner),
        }
    }
    pub async fn start(plugins: Vec<ModPlugin>, host: Arc<dyn ModHost>) -> ModResult<Self> {
        let (sender, receiver) = mpsc::channel();
        let (host_sender, mut host_receiver) =
            tokio::sync::mpsc::unbounded_channel::<crate::bridge::HostRequest>();
        let (ready, initialized) = oneshot::channel();
        let stopping = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let start = crate::actor::Start {
            stopping: Arc::clone(&stopping),
            plugins,
            receiver,
            sender: sender.clone(),
            host: host_sender,
            ready,
            runtime: tokio::runtime::Handle::current(),
        };
        std::thread::Builder::new()
            .name("codex-claude-mods".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(move || crate::actor::run(start))
            .map_err(|error| error.to_string())?;
        tokio::spawn(async move {
            while let Some(request) = host_receiver.recv().await {
                let host = Arc::clone(&host);
                tokio::spawn(async move {
                    let outcome = tokio::select! {
                        outcome = host.call(request.call) => outcome,
                        _ = request.cancellation.cancelled() => Err("mod host call cancelled".into()),
                    };
                    let _ = request.reply.send(outcome);
                });
            }
        });
        let isolate = initialized
            .await
            .map_err(|_| "mod runtime failed to initialize".to_string())??;
        Ok(Self {
            inner: Arc::new(RuntimeHandle {
                sender,
                sequence: AtomicU64::new(1),
                isolate,
                stopping,
            }),
        })
    }

    pub async fn dispatch<Next, Output>(&self, event: Dispatch, mut next: Next) -> ModResult<Value>
    where
        Next: FnMut(Value) -> Output,
        Output: Future<Output = ModResult<Value>>,
    {
        let id = self.inner.sequence.fetch_add(1, Ordering::Relaxed);
        let (result, mut completed) = oneshot::channel();
        let (next_sender, mut continued) = tokio::sync::mpsc::unbounded_channel();
        let _guard = DispatchGuard {
            id,
            sender: self.inner.sender.clone(),
        };
        let streaming = event.name == "turn.step";
        self.inner
            .sender
            .send(Command::Dispatch {
                id,
                name: event.name,
                input: event.input,
                pending: PendingDispatch {
                    cancellation: tokio_util::sync::CancellationToken::new(),
                    next: next_sender,
                    result,
                },
            })
            .map_err(|_| "mod runtime stopped".to_string())?;
        let mut deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
        loop {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline), if !streaming => return Err("mod hook exceeded its 60 second asynchronous deadline".into()),
                outcome = &mut completed => return outcome.map_err(|_| "mod runtime dropped its event".to_string())?,
                request = continued.recv() => {
                    if let Some(request) = request {
                        let started = tokio::time::Instant::now();
                        let outcome = next(request.input).await;
                        deadline += started.elapsed();
                        let _ = request.reply.send(outcome);
                    } else {
                        return completed.await.map_err(|_| "mod event stopped".to_string())?;
                    }
                }
            }
        }
    }

    pub async fn observe(&self, event: Dispatch) -> ModResult<Value> {
        self.dispatch(event, |input| async move { Ok(input) }).await
    }

    pub async fn inspect(&self, method: &str) -> ModResult<Value> {
        let (reply, received) = oneshot::channel();
        self.inner
            .sender
            .send(Command::Inspect {
                method: method.into(),
                reply,
            })
            .map_err(|_| "mod runtime stopped".to_string())?;
        received
            .await
            .map_err(|_| "mod inspection stopped".to_string())?
    }
}
