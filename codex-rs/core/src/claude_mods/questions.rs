use super::host::step;
use super::host::string;
use crate::session::session::Session;
use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::ModResult;
use codex_protocol::request_user_input::RequestUserInputArgs;
use codex_protocol::request_user_input::RequestUserInputQuestion;
use codex_protocol::request_user_input::RequestUserInputQuestionOption;
use serde_json::Value;
use std::sync::Arc;

pub(crate) async fn ask(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    if session.active_turn.lock().await.is_none() {
        return Err("mod questions require an active turn".into());
    }
    let step = step(session).await?;
    if matches!(
        step.turn.session_source,
        codex_protocol::protocol::SessionSource::SubAgent(_)
    ) {
        return Err("subagent mods cannot ask the user questions".into());
    }
    let question = string(call.arguments.first().unwrap_or(&Value::Null))?;
    let options = call.arguments.get(1).unwrap_or(&Value::Null);
    let choices = options["options"]
        .as_array()
        .ok_or_else(|| "mod question requires options".to_string())?
        .iter()
        .map(|choice| {
            string(choice).map(|label| RequestUserInputQuestionOption {
                label: label.into(),
                description: label.into(),
            })
        })
        .collect::<ModResult<Vec<_>>>()?;
    if choices.is_empty() || choices.len() > 9 {
        return Err("mod questions require 1 to 9 options".into());
    }
    let id = format!("mod-{}", uuid::Uuid::new_v4());
    let accepted = session
        .request_user_input(
            &step.turn,
            id.clone(),
            RequestUserInputArgs {
                questions: vec![RequestUserInputQuestion {
                    id: id.clone(),
                    header: options["header"].as_str().unwrap_or("Mod").into(),
                    question: question.into(),
                    is_other: true,
                    is_secret: false,
                    options: Some(choices),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        )
        .await
        .ok_or_else(|| "mod question cancelled".to_string())?;
    let answer = accepted
        .response
        .answers
        .get(&id)
        .and_then(|answer| answer.answers.first())
        .cloned();
    Ok(answer.map(Value::String).unwrap_or(Value::Null))
}
