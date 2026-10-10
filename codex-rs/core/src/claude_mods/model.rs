use super::host::step;
use super::model_options::ModelOptions;
use crate::client_common::Prompt;
use crate::client_common::ResponseEvent;
use crate::responses_metadata::CodexResponsesRequestKind;
use crate::session::session::Session;
use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::ModResult;
use codex_protocol::config_types::ReasoningSummary;
use codex_protocol::models::BaseInstructions;
use codex_protocol::openai_models::ReasoningEffort;
use codex_rollout_trace::InferenceTraceContext;
use futures::StreamExt;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

pub(crate) async fn complete(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let options = ModelOptions::parse(
        call.arguments
            .first()
            .cloned()
            .ok_or_else(|| "model call requires options".to_string())?,
    )?;
    let timeout = options.timeout_ms.unwrap_or(30000);
    tokio::time::timeout(
        Duration::from_millis(timeout),
        request(session, call, &options),
    )
    .await
    .map_err(|_| format!("mod model call exceeded {timeout} ms"))?
}

async fn request(
    session: &Arc<Session>,
    call: &HostCall,
    options: &ModelOptions,
) -> ModResult<Value> {
    let step = step(session).await?;
    let requested = options.model.as_deref().unwrap_or("codex:active");
    let model = if requested == "codex:active" {
        Arc::clone(&step.settings.model_info)
    } else {
        Arc::new(
            session
                .services
                .models_manager
                .get_model_info(requested, &step.turn.config.to_models_manager_config())
                .await,
        )
    };
    let effort = options.effort.clone().or(Some(ReasoningEffort::Low));
    let maximum = options.max_tokens.unwrap_or(4096);
    let mut prompt = Prompt {
        max_output_tokens: Some(maximum),
        base_instructions: BaseInstructions {
            provenance: None,
            text: options
                .system
                .as_deref()
                .unwrap_or("Answer the request. Return only the requested output.")
                .into(),
        },
        ..Prompt::default()
    };
    if call.method == "model.fork" {
        prompt.input = session
            .conversation_history_snapshot()
            .await
            .items()
            .cloned()
            .collect();
    }
    prompt
        .input
        .push(super::message("user", options.prompt.clone()));
    let metadata = session
        .responses_metadata(&step, CodexResponsesRequestKind::Turn)
        .await;
    let mut client = session.services.model_client.new_session();
    let started = std::time::Instant::now();
    let mut stream = client
        .stream(
            &prompt,
            &model,
            &step.session_telemetry,
            effort,
            ReasoningSummary::None,
            step.turn.config.service_tier.clone(),
            &metadata,
            &InferenceTraceContext::disabled(),
        )
        .await
        .map_err(|error| error.to_string())?;
    let mut text = String::new();
    let mut completed_text = String::new();
    while let Some(event) = stream.next().await {
        match event.map_err(|error| error.to_string())? {
            ResponseEvent::OutputTextDelta(delta) => text.push_str(&delta),
            ResponseEvent::OutputItemDone(codex_protocol::models::ResponseItem::Message {
                role,
                content,
                ..
            }) if role == "assistant" => {
                completed_text.push_str(
                    &content
                        .into_iter()
                        .filter_map(|part| match part {
                            codex_protocol::models::ContentItem::OutputText { text } => Some(text),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
            ResponseEvent::OutputItemDone(codex_protocol::models::ResponseItem::AgentMessage {
                content,
                ..
            }) => {
                completed_text.push_str(
                    &codex_protocol::models::plaintext_agent_message_content(&content)
                        .unwrap_or_default(),
                );
            }
            ResponseEvent::Completed { token_usage, .. } => {
                if !completed_text.is_empty() {
                    text = completed_text;
                }
                return Ok(
                    json!({"isAnswered":!text.is_empty(),"text":text,"model":model.slug,"usage":token_usage,"durationMs":started.elapsed().as_millis()}),
                );
            }
            _ => {}
        }
    }
    Err("mod model response ended before completion".into())
}
