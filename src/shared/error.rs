use std::path::PathBuf;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Ai(#[from] AiError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Infra(#[from] InfraError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    TomlDe(#[from] toml::de::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Error)]
pub enum AiError {
    #[error("AI is disabled in the current configuration")]
    Disabled,
    #[error("AI provider is not configured")]
    ProviderNotConfigured,
    #[error("unsupported AI provider `{0}`")]
    UnsupportedProvider(String),
    #[error("missing API key for provider `{provider}` in `${env_var}`")]
    MissingApiKey { provider: String, env_var: String },
    #[error("AI provider `{provider}` is unavailable: {reason}")]
    ProviderUnavailable { provider: String, reason: String },
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse config at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

#[derive(Debug, Error)]
pub enum InfraError {
    #[error("failed to spawn command `{command}`: {source}")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("git inspection failed for {path}: {message}")]
    Git { path: PathBuf, message: String },
    #[error("project scan failed for {path}: {message}")]
    Project { path: PathBuf, message: String },
    #[error("ai provider `{provider}` failed: {message}")]
    Ai { provider: String, message: String },
    #[error("terminal failure: {0}")]
    Terminal(String),
    #[error("runtime failure: {0}")]
    Runtime(String),
}

impl AppError {
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}
