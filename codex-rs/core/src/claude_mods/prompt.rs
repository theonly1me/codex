use crate::WithTurnExtensionData;
use crate::session::session::Session;
use codex_protocol::error::CodexErr;
use codex_protocol::protocol::AdditionalContextEntry;
use codex_protocol::protocol::AdditionalContextKind;
use codex_protocol::turn_input::TurnInput;
use codex_protocol::turn_input::TurnInputRequest;
use codex_protocol::user_input::UserInput;
use serde_json::json;

pub(crate) async fn submit(
    session: &Session,
    request: &mut WithTurnExtensionData<TurnInputRequest>,
) -> Result<bool, CodexErr> {
    session
        .services
        .claude_mods
        .check()
        .map_err(CodexErr::InvalidRequest)?;
    if session.services.claude_mods.runtime().is_none() {
        return Ok(true);
    }
    if request
        .request
        .start
        .turn_trigger
        .as_deref()
        .is_some_and(|trigger| trigger.starts_with("claude-mod:"))
    {
        return Ok(true);
    }
    let TurnInput::UserInput { content, .. } = &mut request.request.input else {
        return Ok(true);
    };
    let text = content
        .iter()
        .filter_map(|item| match item {
            UserInput::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut input = json!({"text":text,"origin":{"kind":"composer"}});
    if let Some(task) = session
        .active_turn
        .lock()
        .await
        .as_ref()
        .and_then(|active| active.task.as_ref())
    {
        input["turnId"] = json!(task.turn_context.sub_id);
    }
    let turn = session.new_default_turn().await;
    let input = super::lifecycle::scoped(&turn, input);
    let event = session
        .services
        .claude_mods
        .observe("prompt.submit", input)
        .await
        .map_err(CodexErr::InvalidRequest)?;
    if let Some(deny) = event["deny"].as_str() {
        return Err(CodexErr::InvalidRequest(format!(
            "Claude mod denied the prompt: {deny}"
        )));
    }
    if event["drop"].as_bool() == Some(true) || event["drop"].is_string() {
        return Ok(false);
    }
    if let Some(changed) = event["text"].as_str()
        && changed != text
    {
        content.retain(|item| !matches!(item, UserInput::Text { .. }));
        content.insert(
            0,
            UserInput::Text {
                text: changed.into(),
                text_elements: Vec::new(),
            },
        );
    }
    if let Some(context) = event["context"].as_array() {
        for (index, text) in context.iter().enumerate() {
            let text = text.as_str().ok_or_else(|| {
                CodexErr::InvalidRequest("mod prompt context requires text".into())
            })?;
            request.request.additional_context.insert(
                format!("claude-mod:{index}"),
                AdditionalContextEntry {
                    value: text.into(),
                    kind: AdditionalContextKind::Untrusted,
                },
            );
        }
    }
    Ok(true)
}
