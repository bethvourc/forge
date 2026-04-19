mod mock;
mod openai;
mod unconfigured;

use std::sync::Arc;

use crate::ai::provider::AiProvider;
use crate::config::AiConfig;

pub fn build_provider(config: &AiConfig) -> Arc<dyn AiProvider> {
    match (config.enabled, config.provider.as_deref()) {
        (false, _) => Arc::new(unconfigured::UnconfiguredProvider::disabled()),
        (true, Some("mock")) => Arc::new(mock::MockProvider::new(config.model.clone())),
        (true, Some("openai")) => Arc::new(openai::OpenAiProvider::new(config)),
        (true, Some(provider)) => {
            Arc::new(unconfigured::UnconfiguredProvider::unsupported(provider))
        }
        (true, None) => Arc::new(unconfigured::UnconfiguredProvider::missing()),
    }
}
