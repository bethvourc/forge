use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::ai::provider::{AiFuture, AiProvider};
use crate::config::AiConfig;
use crate::domain::{AiCitation, AiRequest, AiResponse};
use crate::shared::error::AiError;
use crate::shared::time::now_utc;

const DEFAULT_MODEL: &str = "gpt-5-mini";
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1/responses";

pub struct OpenAiProvider {
    client: Client,
    api_key: Option<String>,
    endpoint: String,
    default_model: String,
}

impl OpenAiProvider {
    pub fn new(config: &AiConfig) -> Self {
        let timeout = Duration::from_secs(config.request_timeout_secs);
        let client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        let endpoint = std::env::var("OPENAI_BASE_URL")
            .or_else(|_| std::env::var("OPENAI_API_BASE"))
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let endpoint = normalize_endpoint(&endpoint);

        Self {
            client,
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            endpoint,
            default_model: config
                .model
                .clone()
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }
}

impl AiProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn execute(&self, request: AiRequest) -> AiFuture {
        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let endpoint = self.endpoint.clone();
        let default_model = self.default_model.clone();

        Box::pin(async move {
            let api_key = api_key.ok_or(AiError::MissingApiKey {
                provider: "openai".to_string(),
                env_var: "OPENAI_API_KEY".to_string(),
            })?;

            let payload = build_payload(&request, &default_model);
            let response = client
                .post(endpoint)
                .bearer_auth(api_key)
                .json(&payload)
                .send()
                .await
                .map_err(|error| AiError::ProviderUnavailable {
                    provider: "openai".to_string(),
                    reason: error.to_string(),
                })?;

            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|error| AiError::ProviderUnavailable {
                    provider: "openai".to_string(),
                    reason: error.to_string(),
                })?;

            if !status.is_success() {
                return Err(AiError::ProviderUnavailable {
                    provider: "openai".to_string(),
                    reason: format!("HTTP {status}: {}", truncate(&body, 240)),
                }
                .into());
            }

            let parsed: OpenAiResponseEnvelope =
                serde_json::from_str(&body).map_err(|error| AiError::ProviderUnavailable {
                    provider: "openai".to_string(),
                    reason: format!("invalid OpenAI response body: {error}"),
                })?;

            let output_text =
                extract_output_text(&parsed).ok_or_else(|| AiError::ProviderUnavailable {
                    provider: "openai".to_string(),
                    reason: "OpenAI response did not contain text output".to_string(),
                })?;

            let structured: StructuredAiResponse =
                serde_json::from_str(&output_text).map_err(|error| {
                    AiError::ProviderUnavailable {
                        provider: "openai".to_string(),
                        reason: format!(
                            "OpenAI response was not valid structured JSON: {error}; body={}",
                            truncate(&output_text, 240)
                        ),
                    }
                })?;

            Ok(AiResponse {
                request_id: request.id,
                created_at: now_utc(),
                provider: "openai".to_string(),
                model: parsed.model.or(request.model.clone()),
                summary: structured.summary,
                message: structured.message,
                recommendations: structured.recommendations,
                proposals: Vec::new(),
                citations: structured
                    .citations
                    .into_iter()
                    .map(|citation| AiCitation {
                        label: citation.label,
                        detail: citation.detail,
                    })
                    .collect(),
            })
        })
    }
}

fn build_payload(request: &AiRequest, default_model: &str) -> Value {
    let model = request
        .model
        .clone()
        .unwrap_or_else(|| default_model.to_string());
    let developer_prompt = build_developer_prompt(request);

    json!({
        "model": model,
        "input": [
            {
                "role": "developer",
                "content": developer_prompt,
            },
            {
                "role": "user",
                "content": request.prompt,
            }
        ],
        "text": {
            "format": {
                "type": "json_schema",
                "name": "forge_ai_response",
                "strict": true,
                "schema": response_schema()
            }
        }
    })
}

fn build_developer_prompt(request: &AiRequest) -> String {
    format!(
        concat!(
            "You are Forge, a terminal-native DevOps command center assistant.\n",
            "Use the provided repository/runtime context. Be concrete, grounded, and concise.\n",
            "Do not claim certainty when evidence is weak.\n",
            "Do not propose hidden actions.\n",
            "Return JSON matching the requested schema.\n\n",
            "Request kind: {}\n",
            "UI focus: {}\n",
            "Dashboard tab: {}\n",
            "Repo: {}\n",
            "Project root: {}\n",
            "Git branch: {}\n",
            "Git head: {}\n",
            "Git dirty: {}\n",
            "Changed files: {}\n",
            "Recent commands:\n{}\n",
            "Recent services:\n{}\n",
            "Recent tests:\n{}\n",
            "Recent logs:\n{}\n",
            "Recent timeline:\n{}\n"
        ),
        request.kind.label(),
        request.context.ui.focus,
        request.context.ui.dashboard_tab,
        request.context.project.name,
        request.context.project.root.display(),
        request
            .context
            .git
            .branch
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        request
            .context
            .git
            .head
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        request.context.git.is_dirty,
        render_list(
            &request
                .context
                .git
                .changed_files
                .iter()
                .map(|file| file.as_str())
                .collect::<Vec<_>>()
        ),
        render_list(
            &request
                .context
                .commands
                .iter()
                .map(|command| format!("#{} {:?} {}", command.id, command.status, command.raw))
                .collect::<Vec<_>>()
        ),
        render_list(
            &request
                .context
                .services
                .iter()
                .map(|service| format!(
                    "{} {:?} pid={:?}",
                    service.name, service.health, service.pid
                ))
                .collect::<Vec<_>>()
        ),
        render_list(
            &request
                .context
                .tests
                .iter()
                .map(|test| format!(
                    "{} {:?} pass={} fail={}",
                    test.runner, test.status, test.pass_count, test.fail_count
                ))
                .collect::<Vec<_>>()
        ),
        render_list(
            &request
                .context
                .logs
                .iter()
                .map(|log| format!(
                    "[{:?}] {} {}",
                    log.severity,
                    log.source_label,
                    truncate(&log.raw, 160)
                ))
                .collect::<Vec<_>>()
        ),
        render_list(
            &request
                .context
                .timeline
                .iter()
                .map(|event| format!("[{:?}] {}", event.kind, truncate(&event.message, 160)))
                .collect::<Vec<_>>()
        )
    )
}

fn render_list<T: ToString>(items: &[T]) -> String {
    if items.is_empty() {
        "- none".to_string()
    } else {
        items
            .iter()
            .map(|item| format!("- {}", item.to_string()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn response_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["summary", "message", "recommendations", "citations"],
        "properties": {
            "summary": {
                "type": "string",
                "description": "A short one-line grounded summary."
            },
            "message": {
                "type": "string",
                "description": "A concise but complete explanation or diagnosis for the user."
            },
            "recommendations": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Concrete next steps or checks."
            },
            "citations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["label", "detail"],
                    "properties": {
                        "label": { "type": "string" },
                        "detail": { "type": "string" }
                    }
                },
                "description": "Short references to the context used."
            }
        }
    })
}

fn normalize_endpoint(raw: &str) -> String {
    let trimmed = raw.trim_end_matches('/');
    if trimmed.ends_with("/responses") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/responses")
    }
}

fn extract_output_text(response: &OpenAiResponseEnvelope) -> Option<String> {
    if let Some(text) = response.output_text.as_ref() {
        if !text.trim().is_empty() {
            return Some(text.clone());
        }
    }

    let combined = response
        .output
        .as_ref()?
        .iter()
        .flat_map(|item| item.content.iter())
        .filter(|content| content.kind == "output_text")
        .map(|content| content.text.as_str())
        .collect::<Vec<_>>()
        .join("");

    (!combined.trim().is_empty()).then_some(combined)
}

fn truncate(value: &str, max_chars: usize) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return value.to_string();
    }
    let visible = chars
        .into_iter()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{visible}…")
}

#[derive(Debug, Deserialize)]
struct OpenAiResponseEnvelope {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    output: Option<Vec<OpenAiOutputItem>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiOutputItem {
    #[serde(default)]
    content: Vec<OpenAiContentItem>,
}

#[derive(Debug, Deserialize)]
struct OpenAiContentItem {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct StructuredAiResponse {
    summary: String,
    message: String,
    recommendations: Vec<String>,
    citations: Vec<StructuredCitation>,
}

#[derive(Debug, Deserialize)]
struct StructuredCitation {
    label: String,
    detail: String,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::ai::providers::openai::{build_payload, extract_output_text, StructuredAiResponse};
    use crate::config::ForgeConfig;
    use crate::domain::{AiRequest, AiRequestKind, AppState, GitSnapshot, ProjectContext};
    use crate::shared::ids::AiRequestId;
    use crate::shared::time::now_utc;

    #[test]
    fn extracts_output_text_from_output_items() {
        let response = serde_json::from_value(json!({
            "model": "gpt-5-mini",
            "output": [
                {
                    "content": [
                        { "type": "output_text", "text": "{\"summary\":\"s\",\"message\":\"m\",\"recommendations\":[],\"citations\":[]}" }
                    ]
                }
            ]
        }))
        .expect("response should deserialize");

        let text = extract_output_text(&response).expect("text should exist");
        let parsed: StructuredAiResponse =
            serde_json::from_str(&text).expect("structured text should parse");
        assert_eq!(parsed.summary, "s");
    }

    #[test]
    fn build_payload_uses_json_schema_format() {
        let config = ForgeConfig::default();
        let state = AppState::new(config, ProjectContext::default(), GitSnapshot::default());
        let request = AiRequest {
            id: AiRequestId(1),
            kind: AiRequestKind::Assist,
            created_at: now_utc(),
            provider: Some("openai".to_string()),
            model: Some("gpt-5-mini".to_string()),
            prompt: "Summarize this repo".to_string(),
            context: crate::ai::build_context(&state),
        };

        let payload = build_payload(&request, "gpt-5-mini");
        assert_eq!(payload["text"]["format"]["type"], "json_schema");
        assert_eq!(payload["model"], "gpt-5-mini");
    }
}
