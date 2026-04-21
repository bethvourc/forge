use crate::ai::provider::{AiFuture, AiProvider};
use crate::domain::AiRequest;
use crate::shared::error::AiError;

pub struct UnconfiguredProvider {
    name: String,
    message: String,
}

impl UnconfiguredProvider {
    pub fn disabled() -> Self {
        Self {
            name: "disabled".to_string(),
            message: "AI is disabled in the Forge configuration.".to_string(),
        }
    }

    pub fn missing() -> Self {
        Self {
            name: "unconfigured".to_string(),
            message:
                "AI provider is not configured yet. Set `ai.provider` to a supported provider."
                    .to_string(),
        }
    }

    pub fn unsupported(provider: &str) -> Self {
        Self {
            name: provider.to_string(),
            message: format!("AI provider `{provider}` is not implemented yet in this build."),
        }
    }
}

impl AiProvider for UnconfiguredProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, _request: AiRequest) -> AiFuture {
        let provider = self.name.clone();
        let message = self.message.clone();
        Box::pin(async move {
            Err(AiError::ProviderUnavailable {
                provider,
                reason: message,
            }
            .into())
        })
    }
}
