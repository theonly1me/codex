use crate::ModPlugin;
use crate::ModResult;
use crate::bridge::Bridge;
use crate::bridge::HostRequest;
use crate::bridge::PendingDispatch;
use crate::engine;
use crate::module_loader::ModuleLoader;
use crate::value::encode;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::oneshot;

const INITIALIZATION_BUDGET_MILLISECONDS: u64 = 10_000;
const EVENT_BUDGET_MILLISECONDS: u64 = 1_000;

pub(crate) enum Command {
    Dispatch {
        id: u64,
        name: String,
        input: Value,
        pending: PendingDispatch,
    },
    Resolve {
        call_id: u64,
        response: ModResult<Value>,
    },
    Cancel {
        id: u64,
    },
    Inspect {
        method: String,
        reply: oneshot::Sender<ModResult<Value>>,
    },
    Shutdown,
}

pub(crate) struct Start {
    pub stopping: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub plugins: Vec<ModPlugin>,
    pub receiver: mpsc::Receiver<Command>,
    pub sender: mpsc::Sender<Command>,
    pub host: tokio::sync::mpsc::UnboundedSender<HostRequest>,
    pub ready: oneshot::Sender<ModResult<v8::IsolateHandle>>,
    pub runtime: tokio::runtime::Handle,
}

pub(crate) fn run(start: Start) {
    if let Err(error) = codex_code_mode_runtime::ensure_v8_initialized() {
        let _ = start.ready.send(Err(error));
        return;
    }
    let isolate = &mut v8::Isolate::new(
        v8::CreateParams::default().heap_limits(16 * 1024 * 1024, 256 * 1024 * 1024),
    );
    crate::heap::install(isolate);
    isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
    let handle = isolate.thread_safe_handle();
    let budget = crate::watchdog::CpuBudget::new(handle.clone());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, v8::ContextOptions::default());
    let scope = &mut v8::ContextScope::new(scope, context);
    scope.set_slot(ModuleLoader {
        root: Default::default(),
        modules: HashMap::new(),
        paths: HashMap::new(),
    });
    scope.set_slot(Bridge {
        cancellation: tokio_util::sync::CancellationToken::new(),
        host: start.host,
        dispatches: HashMap::new(),
        promises: HashMap::new(),
        next_call_id: 1,
        replies: start.sender,
        runtime: start.runtime,
    });
    budget.arm(INITIALIZATION_BUDGET_MILLISECONDS);
    let initialized = engine::caught(scope, |scope| {
        engine::install(scope)?;
        for plugin in &start.plugins {
            engine::load(scope, plugin)?;
        }
        Ok(())
    });
    budget.clear();
    if let Err(error) = initialized {
        let _ = start.ready.send(Err(error));
        return;
    }
    if start.ready.send(Ok(handle)).is_err() {
        return;
    }
    loop {
        let command = match start.receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(Command::Shutdown) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Ok(command) => Some(command),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
        };
        budget.arm(EVENT_BUDGET_MILLISECONDS);
        let outcome = engine::caught(scope, |scope| {
            if let Some(command) = command {
                process(scope, command)?;
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|error| error.to_string())?
                .as_millis() as f64;
            let now = v8::Number::new(scope, now);
            engine::call(scope, "tick", &[now.into()])?;
            scope.perform_microtask_checkpoint();
            Ok(())
        });
        budget.clear();
        if start.stopping.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        if crate::heap::exceeded() {
            fail_pending(scope, "mod exceeded its 256 MiB JavaScript heap limit");
            break;
        }
        if budget.exceeded.load(std::sync::atomic::Ordering::Acquire) {
            fail_pending(scope, "mod exceeded its JavaScript execution deadline");
            break;
        }
        if let Err(error) = outcome {
            fail_pending(scope, &error);
            tracing::error!(%error, "Claude mod runtime stopped");
            break;
        }
    }
    start
        .stopping
        .store(true, std::sync::atomic::Ordering::Release);
    fail_pending(scope, "mod runtime stopped");
}

fn process(scope: &mut v8::PinScope<'_, '_>, command: Command) -> ModResult<()> {
    match command {
        Command::Dispatch {
            id,
            name,
            input,
            pending,
        } => {
            let Some(bridge) = scope.get_slot_mut::<Bridge>() else {
                return Err("mod bridge stopped".into());
            };
            bridge.dispatches.insert(id, pending);
            let id = v8::Number::new(scope, id as f64);
            let name = v8::String::new(scope, &name)
                .ok_or_else(|| "cannot allocate event name".to_string())?;
            let input = encode(scope, &input)?;
            engine::call(scope, "begin", &[id.into(), name.into(), input])?;
        }
        Command::Resolve { call_id, response } => {
            engine::resolve_promise(scope, call_id, response)?
        }
        Command::Cancel { id } => {
            if let Some(bridge) = scope.get_slot_mut::<Bridge>()
                && let Some(dispatch) = bridge.dispatches.remove(&id)
            {
                dispatch.cancellation.cancel();
            }
            let id = v8::Number::new(scope, id as f64);
            engine::call(scope, "abort", &[id.into()])?;
        }
        Command::Inspect { method, reply } => {
            let _ = reply.send(engine::inspect(scope, &method));
        }
        Command::Shutdown => {}
    }
    Ok(())
}

fn fail_pending(scope: &mut v8::PinScope<'_, '_>, error: &str) {
    if let Some(bridge) = scope.get_slot_mut::<Bridge>() {
        bridge.cancellation.cancel();
        for (_, dispatch) in bridge.dispatches.drain() {
            dispatch.cancellation.cancel();
            let _ = dispatch.result.send(Err(error.to_string()));
        }
        bridge.promises.clear();
    }
}
