use crate::ModResult;
use futures::future::BoxFuture;
use serde_json::Value;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[derive(Debug)]
pub struct HostCall {
    pub plugin: String,
    pub method: String,
    pub arguments: Vec<Value>,
}

pub trait ModHost: Send + Sync + 'static {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>>;
}

pub(crate) struct NextRequest {
    pub input: Value,
    pub reply: oneshot::Sender<ModResult<Value>>,
}

pub(crate) struct HostRequest {
    pub cancellation: tokio_util::sync::CancellationToken,
    pub call: HostCall,
    pub reply: oneshot::Sender<ModResult<Value>>,
}

pub(crate) struct PendingDispatch {
    pub cancellation: tokio_util::sync::CancellationToken,
    pub next: mpsc::UnboundedSender<NextRequest>,
    pub result: oneshot::Sender<ModResult<Value>>,
}

pub(crate) struct Bridge {
    pub cancellation: tokio_util::sync::CancellationToken,
    pub host: mpsc::UnboundedSender<HostRequest>,
    pub dispatches: HashMap<u64, PendingDispatch>,
    pub promises: HashMap<u64, v8::Global<v8::PromiseResolver>>,
    pub next_call_id: u64,
    pub replies: std::sync::mpsc::Sender<crate::actor::Command>,
    pub runtime: tokio::runtime::Handle,
}

pub(crate) fn host_callback(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    mut result: v8::ReturnValue<v8::Value>,
) {
    let request = crate::value::decode(scope, arguments.get(0));
    let resolver = v8::PromiseResolver::new(scope);
    let (Ok(request), Some(resolver)) = (request, resolver) else {
        return;
    };
    let promise = resolver.get_promise(scope);
    result.set(promise.into());
    let plugin = request["plugin"].as_str().unwrap_or_default().to_string();
    let method = request["method"].as_str().unwrap_or_default().to_string();
    let dispatch_id = request["dispatchId"].as_u64().unwrap_or_default();
    let inputs = request["arguments"].as_array().cloned().unwrap_or_default();
    let resolver = v8::Global::new(scope, resolver);
    let Some(bridge) = scope.get_slot_mut::<Bridge>() else {
        return;
    };
    let call_id = bridge.next_call_id;
    bridge.next_call_id += 1;
    bridge.promises.insert(call_id, resolver);
    let (reply, received) = oneshot::channel();
    let sent = if method == "__next" {
        bridge
            .dispatches
            .get(&dispatch_id)
            .ok_or_else(|| "mod event is no longer active".to_string())
            .and_then(|dispatch| {
                dispatch
                    .next
                    .send(NextRequest {
                        input: inputs.into_iter().next().unwrap_or(Value::Null),
                        reply,
                    })
                    .map_err(|_| "mod continuation was cancelled".to_string())
            })
    } else {
        let cancellation = bridge
            .dispatches
            .get(&dispatch_id)
            .map(|dispatch| dispatch.cancellation.clone())
            .unwrap_or_else(|| bridge.cancellation.clone());
        bridge
            .host
            .send(HostRequest {
                cancellation,
                call: HostCall {
                    plugin,
                    method,
                    arguments: inputs,
                },
                reply,
            })
            .map_err(|_| "mod host stopped".to_string())
    };
    let replies = bridge.replies.clone();
    bridge.runtime.spawn(async move {
        let response = match sent {
            Ok(()) => received
                .await
                .unwrap_or_else(|_| Err("mod host dropped its response".into())),
            Err(error) => Err(error),
        };
        let _ = replies.send(crate::actor::Command::Resolve { call_id, response });
    });
}

pub(crate) fn settle_callback(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _result: v8::ReturnValue<v8::Value>,
) {
    let id = arguments.get(0).integer_value(scope).unwrap_or_default() as u64;
    let outcome = crate::value::decode(scope, arguments.get(1));
    let failed = arguments.get(2).is_true();
    let outcome = outcome.and_then(|value| {
        if failed {
            Err(value.as_str().unwrap_or("mod event failed").to_string())
        } else {
            Ok(value)
        }
    });
    if let Some(bridge) = scope.get_slot_mut::<Bridge>()
        && let Some(dispatch) = bridge.dispatches.remove(&id)
    {
        let _ = dispatch.result.send(outcome);
    }
}
