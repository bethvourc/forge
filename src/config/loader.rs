use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::config::schema::{ForgeConfig, PartialForgeConfig};
use crate::shared::error::{AppResult, ConfigError};

#[derive(Debug, Clone, Parser)]
#[command(name = "forge", version, about = "Forge DevOps command center")]
pub struct CliArgs {
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub cwd: Option<PathBuf>,
    #[arg(long)]
    pub debug: bool,
    #[arg(long)]
    pub tick_rate_ms: Option<u64>,
    #[arg(long)]
    pub log_level: Option<String>,
    #[arg(long)]
    pub ai_provider: Option<String>,
    #[arg(long)]
    pub ai_model: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: ForgeConfig,
    pub cwd: PathBuf,
    pub global_config_path: Option<PathBuf>,
    pub project_config_path: Option<PathBuf>,
}

pub fn load_config(cli: &CliArgs) -> AppResult<LoadedConfig> {
    let cwd = match cli.cwd.clone() {
        Some(path) => path,
        None => std::env::current_dir()?,
    };

    let mut config = ForgeConfig::default();

    let global_candidate = dirs::config_dir().map(|dir| dir.join("forge").join("config.toml"));
    let global_config_path =
        if let Some(path) = global_candidate.as_ref().filter(|path| path.exists()) {
            read_partial(path)?.apply_to(&mut config);
            Some(path.clone())
        } else {
            None
        };

    let project_candidate = cli.config.clone().or_else(|| {
        let path = cwd.join(".forge").join("config.toml");
        path.exists().then_some(path)
    });

    if let Some(path) = project_candidate.as_ref() {
        read_partial(path)?.apply_to(&mut config);
    }

    apply_env_overrides(&mut config);
    apply_cli_overrides(cli, &mut config);
    config.normalize();

    Ok(LoadedConfig {
        config,
        cwd,
        global_config_path,
        project_config_path: project_candidate,
    })
}

fn read_partial(path: &Path) -> AppResult<PartialForgeConfig> {
    let raw = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    if raw.trim().is_empty() {
        return Ok(PartialForgeConfig::default());
    }
    toml::from_str(&raw)
        .map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
        .map_err(Into::into)
}

fn apply_env_overrides(config: &mut ForgeConfig) {
    if let Ok(value) = std::env::var("FORGE_TICK_RATE_MS") {
        if let Ok(parsed) = value.parse::<u64>() {
            config.ui.tick_rate_ms = parsed;
        }
    }
    if let Ok(value) = std::env::var("FORGE_LOG_LEVEL") {
        config.observability.log_level = value;
    }
    if let Ok(value) = std::env::var("FORGE_SHELL") {
        config.commands.default_shell = value;
    }
    if let Ok(value) = std::env::var("FORGE_AI_ENABLED") {
        match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => config.ai.enabled = true,
            "0" | "false" | "no" | "off" => config.ai.enabled = false,
            _ => {}
        }
    }
    if let Ok(value) = std::env::var("FORGE_AI_PROVIDER") {
        config.ai.provider = Some(value);
    }
    if let Ok(value) = std::env::var("FORGE_AI_MODEL") {
        config.ai.model = Some(value);
    }
    if let Ok(value) = std::env::var("FORGE_AI_TIMEOUT_SECS") {
        if let Ok(parsed) = value.parse::<u64>() {
            config.ai.request_timeout_secs = parsed;
        }
    }
    if let Ok(value) = std::env::var("FORGE_EVENT_STREAM_HEIGHT") {
        if let Ok(parsed) = value.parse::<u16>() {
            config.ui.event_stream_height = parsed;
        }
    }
    if let Ok(value) = std::env::var("FORGE_MAIN_SPLIT_PCT") {
        if let Ok(parsed) = value.parse::<u16>() {
            config.ui.main_split_pct = parsed;
        }
    }
}

fn apply_cli_overrides(cli: &CliArgs, config: &mut ForgeConfig) {
    if let Some(value) = cli.tick_rate_ms {
        config.ui.tick_rate_ms = value;
    }
    if let Some(value) = cli.log_level.as_ref() {
        config.observability.log_level = value.clone();
    }
    if let Some(value) = cli.ai_provider.as_ref() {
        config.ai.provider = Some(value.clone());
    }
    if let Some(value) = cli.ai_model.as_ref() {
        config.ai.model = Some(value.clone());
    }
    if cli.debug {
        config.observability.log_level = "debug".to_string();
    }
}
