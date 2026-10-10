use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use codex_claude_mods_extension::ModResult;
use serde_json::Value;
use serde_json::json;

pub(crate) async fn events(invocation: &ToolInvocation) -> ModResult<Vec<Value>> {
    let name = invocation.tool_name.name.as_str();
    let input = match &invocation.payload {
        ToolPayload::Function { arguments } => {
            serde_json::from_str::<Value>(arguments).map_err(|error| error.to_string())?
        }
        ToolPayload::Custom { input } => json!({"input":input}),
        _ => json!({}),
    };
    if name == "apply_patch" {
        let patch = input["input"]
            .as_str()
            .or_else(|| input["patch"].as_str())
            .ok_or_else(|| "apply_patch has no patch text".to_string())?;
        let patch = codex_apply_patch::parse_patch(patch).map_err(|error| error.to_string())?;
        let mut events = Vec::new();
        for hunk in patch.hunks {
            events.extend(super::patch_events::events(invocation, hunk).await?);
        }
        return Ok(events);
    }
    if name == "write_stdin"
        && let Some(id) = input["session_id"].as_u64()
        && let Some(mut event) = super::state(&invocation.session).process_event(id).await
    {
        event["session_id"] = json!(id);
        return Ok(vec![event]);
    }
    let tool = match name {
        "exec_command" | "shell" | "shell_command" => "Bash",
        "spawn_agent" => "Agent",
        "read_file" => "Read",
        "grep_files" => "Grep",
        "list_dir" => "Glob",
        _ => name,
    };
    let mut event = input;
    if !event.is_object() {
        event = json!({"input":event})
    }
    event["tool"] = json!(tool);
    event["requestId"] = json!(invocation.call_id);
    event["tool_use_id"] = json!(invocation.call_id);
    event["turnId"] = json!(invocation.turn.sub_id);
    if tool == "Bash" {
        event["command"] = event
            .get("cmd")
            .or_else(|| event.get("command"))
            .cloned()
            .unwrap_or(Value::Null);
        if let Some(arguments) = event["command"].as_array() {
            event["command"] = json!(
                arguments
                    .iter()
                    .map(|value| value
                        .as_str()
                        .map(|value| format!("'{}'", value.replace('\'', "'\\''")))
                        .ok_or_else(|| "shell arguments require text".to_string()))
                    .collect::<ModResult<Vec<_>>>()?
                    .join(" ")
            );
        }
    }
    let mut events = vec![super::lifecycle::scoped(&invocation.turn, event.clone())];
    if tool == "Bash" {
        events.extend(super::shell_patch::events(invocation, &event).await?);
    }
    Ok(events)
}

pub(crate) async fn result(
    invocation: &ToolInvocation,
    event: &Value,
    mut outcome: Value,
) -> Value {
    match event["tool"].as_str() {
        Some("Bash") => {
            let output = outcome
                .get("nativeResult")
                .unwrap_or(&outcome["result"])
                .clone();
            let id = output["session_id"]
                .as_u64()
                .or_else(|| event["session_id"].as_u64());
            let exit = output.get("exit_code").cloned().unwrap_or_else(|| {
                if output["session_id"].is_number() {
                    Value::Null
                } else {
                    json!(if outcome["isError"] == true { 1 } else { 0 })
                }
            });
            if let Some(id) = id {
                if exit.is_null() {
                    super::state(&invocation.session)
                        .record_process(id, event.clone())
                        .await;
                } else {
                    super::state(&invocation.session).forget_process(id).await;
                }
            }
            outcome["result"] = json!({"stdout":output["output"].as_str().or_else(|| output.as_str()).unwrap_or_default(),"stderr":"","exitCode":exit,"interrupted":false});
        }
        Some("Edit") | Some("Write") => {
            outcome["result"] = json!({"structuredPatch":event["structuredPatch"],"type":event["type"],"originalFile":event["originalFile"],"staged":false});
        }
        _ => {}
    }
    outcome
}
