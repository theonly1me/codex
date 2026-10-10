use crate::client_common::ResponseEvent;
use codex_protocol::models::ResponseItem;
use serde_json::Value;
use serde_json::json;

pub(super) fn chunk(event: &ResponseEvent) -> Value {
    match event {
        ResponseEvent::OutputTextDelta(text) => json!({"kind":"text","text":text}),
        ResponseEvent::ReasoningSummaryDelta { delta, .. }
        | ResponseEvent::ReasoningContentDelta { delta, .. } => {
            json!({"kind":"thinking","text":delta})
        }
        ResponseEvent::OutputItemAdded(ResponseItem::FunctionCall {
            call_id,
            name,
            arguments,
            ..
        }) => {
            json!({"kind":"tool","id":call_id,"tool":name,"input":serde_json::from_str::<Value>(arguments).unwrap_or(Value::Null)})
        }
        ResponseEvent::OutputItemAdded(ResponseItem::CustomToolCall {
            call_id,
            name,
            input,
            ..
        }) => json!({"kind":"tool","id":call_id,"tool":name,"input":input}),
        _ => json!({"kind":"native"}),
    }
}
