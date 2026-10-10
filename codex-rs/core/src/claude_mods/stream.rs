use crate::client_common::ResponseEvent;
use crate::client_common::ResponseStream;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use codex_claude_mods_extension::Dispatch;
use codex_protocol::error::CodexErr;
use codex_protocol::models::ResponseItem;
use futures::StreamExt;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

struct State {
    stream: ResponseStream,
    pending: HashMap<u64, ResponseEvent>,
    sequence: u64,
    text: String,
    completed: bool,
    block_index: u64,
}

pub(crate) fn wrap(
    session: &Session,
    turn: &TurnContext,
    mut stream: ResponseStream,
    cancellation: CancellationToken,
) -> ResponseStream {
    let Some(runtime) = super::state(session).runtime() else {
        return stream;
    };
    let interrupt = stream.interrupt.take();
    let consumer_dropped = CancellationToken::new();
    let stopped = consumer_dropped.clone();
    let (sender, receiver) = mpsc::channel(32);
    let state = Arc::new(Mutex::new(Some(State {
        stream,
        pending: HashMap::new(),
        sequence: 0,
        text: String::new(),
        completed: false,
        block_index: 0,
    })));
    let event = Dispatch {
        name: "turn.step".into(),
        input: super::lifecycle::scoped(turn, json!({"turnId":turn.sub_id})),
    };
    tokio::spawn(async move {
        let final_state = Arc::clone(&state);
        let output = sender.clone();
        let dispatch = runtime.dispatch(event, move |request| {
            let state = Arc::clone(&state);
            let output = output.clone();
            async move {
                let mut owned = {
                    state
                        .lock()
                        .map_err(|_| "mod stream state poisoned".to_string())?
                        .take()
                        .ok_or_else(|| "mod stream is already in use".to_string())?
                };
                let result = handle(&mut owned, &output, request).await;
                *state
                    .lock()
                    .map_err(|_| "mod stream state poisoned".to_string())? = Some(owned);
                result
            }
        });
        let outcome = tokio::select! {
            result = dispatch => result,
            _ = stopped.cancelled() => return,
            _ = cancellation.cancelled() => return,
        };
        let outcome = outcome.and_then(|value| {
            let state = final_state
                .lock()
                .map_err(|_| "mod stream state poisoned".to_string())?;
            let state = state
                .as_ref()
                .ok_or_else(|| "mod stream state disappeared".to_string())?;
            if !state.completed
                || state
                    .pending
                    .values()
                    .any(|event| !matches!(event, ResponseEvent::OutputTextDelta(_)))
            {
                Err("mod must preserve native structural stream events and completion".into())
            } else {
                Ok(value)
            }
        });
        if let Err(error) = outcome {
            let _ = sender
                .send(Err(CodexErr::InvalidRequest(format!(
                    "mod turn.step: {error}"
                ))))
                .await;
        }
    });
    ResponseStream {
        rx_event: receiver,
        interrupt,
        consumer_dropped,
    }
}

async fn handle(
    state: &mut State,
    output: &mpsc::Sender<Result<ResponseEvent, CodexErr>>,
    request: Value,
) -> Result<Value, String> {
    match request["streamOperation"].as_str() {
        Some("start") => Ok(Value::Null),
        Some("pull") => {
            if state.completed {
                return Ok(json!({"done":true,"result":{"text":state.text,"answer":state.text}}));
            }
            let event = state
                .stream
                .next()
                .await
                .ok_or_else(|| "model stream ended without completion".to_string())?
                .map_err(|error| error.to_string())?;
            state.sequence += 1;
            if matches!(
                &event,
                ResponseEvent::OutputItemAdded(ResponseItem::Message { .. })
            ) {
                state.block_index += 1;
            }
            let mut chunk = super::stream_chunk::chunk(&event);
            chunk["nativeIndex"] = json!(state.sequence);
            chunk["index"] = json!(state.block_index);
            if let ResponseEvent::OutputTextDelta(text) = &event {
                state.text.push_str(text);
            }
            state.completed = matches!(event, ResponseEvent::Completed { .. });
            state.pending.insert(state.sequence, event);
            Ok(json!({"done":false,"value":chunk}))
        }
        Some("emit") => {
            let chunk = &request["chunk"];
            let event = if let Some(index) = chunk["nativeIndex"].as_u64() {
                let mut event = state
                    .pending
                    .remove(&index)
                    .ok_or_else(|| "mod emitted a stream chunk twice".to_string())?;
                if let ResponseEvent::OutputTextDelta(text) = &mut event {
                    *text = chunk["text"]
                        .as_str()
                        .ok_or_else(|| "mod text chunk requires text".to_string())?
                        .into();
                }
                event
            } else if chunk["kind"] == "text" {
                ResponseEvent::OutputTextDelta(
                    chunk["text"]
                        .as_str()
                        .ok_or_else(|| "mod text chunk requires text".to_string())?
                        .into(),
                )
            } else {
                return Err("mods can create only text stream chunks".into());
            };
            output
                .send(Ok(event))
                .await
                .map_err(|_| "mod stream consumer stopped".to_string())?;
            Ok(Value::Null)
        }
        _ => Err("unsupported mod stream operation".into()),
    }
}
