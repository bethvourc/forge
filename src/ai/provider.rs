use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::ai::providers;
use crate::config::AiConfig;
use crate::domain::{AiRequest, AiResponse};
use crate::shared::error::AppResult;

pub type AiFuture = Pin<Box<dyn Future<Output = AppResult<AiResponse>> + Send>>;

pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, request: AiRequest) -> AiFuture;
}

#[derive(Clone)]
pub struct AiRuntime {
    provider: Arc<dyn AiProvider>,
}

impl AiRuntime {
    pub fn from_config(config: &AiConfig) -> Self {
        Self {
            provider: providers::build_provider(config),
        }
    }

    pub fn provider_name(&self) -> &str {
        self.provider.name()
    }

    pub async fn execute(&self, request: AiRequest) -> AppResult<AiResponse> {
        self.provider.execute(request).await
    }
}
