use codex_claude_mods_extension::ModResult;
use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ModelOptions {
    pub prompt: String,
    pub system: Option<String>,
    pub model: Option<String>,
    pub effort: Option<ReasoningEffort>,
    pub max_tokens: Option<u64>,
    pub timeout_ms: Option<u64>,
}

impl ModelOptions {
    pub(super) fn parse(value: Value) -> ModResult<Self> {
        let options: Self = serde_json::from_value(value)
            .map_err(|error| format!("invalid mod model options: {error}"))?;
        if options
            .max_tokens
            .is_some_and(|tokens| !(1..=65536).contains(&tokens))
        {
            return Err("mod maxTokens must be between 1 and 65536".into());
        }
        if options
            .timeout_ms
            .is_some_and(|timeout| !(1..=120000).contains(&timeout))
        {
            return Err("mod timeoutMs must be between 1 and 120000".into());
        }
        if options.prompt.is_empty() {
            return Err("mod model prompt must not be empty".into());
        }
        Ok(options)
    }
}
