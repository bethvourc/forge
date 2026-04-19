use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ForgeConfig {
    pub ui: UiConfig,
    pub commands: CommandConfig,
    pub safety: SafetyConfig,
    pub services: ServiceConfig,
    pub logs: LogConfig,
    pub git: GitConfig,
    pub ai: AiConfig,
    pub observability: ObservabilityConfig,
}

impl ForgeConfig {
    pub fn normalize(&mut self) {
        self.ui.tick_rate_ms = self.ui.tick_rate_ms.max(33);
        self.ui.main_split_pct = self.ui.main_split_pct.clamp(25, 75);
        self.ui.event_stream_height = self.ui.event_stream_height.clamp(6, 16);
        self.commands.scrollback_limit = self.commands.scrollback_limit.max(100);
        self.commands.history_limit = self.commands.history_limit.max(25);
        self.logs.global_capacity = self.logs.global_capacity.max(200);
        self.logs.per_source_capacity = self.logs.per_source_capacity.max(50);
        self.ai.request_timeout_secs = self.ai.request_timeout_secs.max(5);
        self.ai.history_limit = self.ai.history_limit.max(5);
        self.ai.max_recent_commands = self.ai.max_recent_commands.max(1);
        self.ai.max_recent_logs = self.ai.max_recent_logs.max(1);
        self.ai.max_recent_events = self.ai.max_recent_events.max(1);
        self.ai.max_recent_services = self.ai.max_recent_services.max(1);
        self.ai.max_recent_tests = self.ai.max_recent_tests.max(1);
        self.ai.max_changed_files = self.ai.max_changed_files.max(1);
        self.observability.diagnostics_capacity = self.observability.diagnostics_capacity.max(100);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    pub tick_rate_ms: u64,
    pub main_split_pct: u16,
    pub event_stream_height: u16,
    pub history_preview: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            tick_rate_ms: 120,
            main_split_pct: 40,
            event_stream_height: 8,
            history_preview: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CommandConfig {
    pub default_shell: String,
    pub scrollback_limit: usize,
    pub history_limit: usize,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            default_shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
            scrollback_limit: 2_000,
            history_limit: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub caution_requires_review: bool,
    pub destructive_requires_confirmation: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            caution_requires_review: true,
            destructive_requires_confirmation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServiceConfig {
    pub health_stale_after_secs: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            health_stale_after_secs: 15,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    pub global_capacity: usize,
    pub per_source_capacity: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            global_capacity: 2_000,
            per_source_capacity: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitConfig {
    pub enabled: bool,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub request_timeout_secs: u64,
    pub history_limit: usize,
    pub max_recent_commands: usize,
    pub max_recent_logs: usize,
    pub max_recent_events: usize,
    pub max_recent_services: usize,
    pub max_recent_tests: usize,
    pub max_changed_files: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            provider: None,
            model: None,
            request_timeout_secs: 30,
            history_limit: 20,
            max_recent_commands: 6,
            max_recent_logs: 12,
            max_recent_events: 12,
            max_recent_services: 6,
            max_recent_tests: 4,
            max_changed_files: 12,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub diagnostics_capacity: usize,
    pub log_to_file: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            diagnostics_capacity: 500,
            log_to_file: true,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialForgeConfig {
    pub ui: PartialUiConfig,
    pub commands: PartialCommandConfig,
    pub safety: PartialSafetyConfig,
    pub services: PartialServiceConfig,
    pub logs: PartialLogConfig,
    pub git: PartialGitConfig,
    pub ai: PartialAiConfig,
    pub observability: PartialObservabilityConfig,
}

impl PartialForgeConfig {
    pub fn apply_to(self, config: &mut ForgeConfig) {
        self.ui.apply_to(&mut config.ui);
        self.commands.apply_to(&mut config.commands);
        self.safety.apply_to(&mut config.safety);
        self.services.apply_to(&mut config.services);
        self.logs.apply_to(&mut config.logs);
        self.git.apply_to(&mut config.git);
        self.ai.apply_to(&mut config.ai);
        self.observability.apply_to(&mut config.observability);
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialUiConfig {
    pub tick_rate_ms: Option<u64>,
    pub main_split_pct: Option<u16>,
    pub event_stream_height: Option<u16>,
    pub history_preview: Option<usize>,
}

impl PartialUiConfig {
    fn apply_to(self, config: &mut UiConfig) {
        if let Some(value) = self.tick_rate_ms {
            config.tick_rate_ms = value;
        }
        if let Some(value) = self.main_split_pct {
            config.main_split_pct = value;
        }
        if let Some(value) = self.event_stream_height {
            config.event_stream_height = value;
        }
        if let Some(value) = self.history_preview {
            config.history_preview = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialCommandConfig {
    pub default_shell: Option<String>,
    pub scrollback_limit: Option<usize>,
    pub history_limit: Option<usize>,
}

impl PartialCommandConfig {
    fn apply_to(self, config: &mut CommandConfig) {
        if let Some(value) = self.default_shell {
            config.default_shell = value;
        }
        if let Some(value) = self.scrollback_limit {
            config.scrollback_limit = value;
        }
        if let Some(value) = self.history_limit {
            config.history_limit = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialSafetyConfig {
    pub caution_requires_review: Option<bool>,
    pub destructive_requires_confirmation: Option<bool>,
}

impl PartialSafetyConfig {
    fn apply_to(self, config: &mut SafetyConfig) {
        if let Some(value) = self.caution_requires_review {
            config.caution_requires_review = value;
        }
        if let Some(value) = self.destructive_requires_confirmation {
            config.destructive_requires_confirmation = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialServiceConfig {
    pub health_stale_after_secs: Option<u64>,
}

impl PartialServiceConfig {
    fn apply_to(self, config: &mut ServiceConfig) {
        if let Some(value) = self.health_stale_after_secs {
            config.health_stale_after_secs = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialLogConfig {
    pub global_capacity: Option<usize>,
    pub per_source_capacity: Option<usize>,
}

impl PartialLogConfig {
    fn apply_to(self, config: &mut LogConfig) {
        if let Some(value) = self.global_capacity {
            config.global_capacity = value;
        }
        if let Some(value) = self.per_source_capacity {
            config.per_source_capacity = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialGitConfig {
    pub enabled: Option<bool>,
}

impl PartialGitConfig {
    fn apply_to(self, config: &mut GitConfig) {
        if let Some(value) = self.enabled {
            config.enabled = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialAiConfig {
    pub enabled: Option<bool>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub request_timeout_secs: Option<u64>,
    pub history_limit: Option<usize>,
    pub max_recent_commands: Option<usize>,
    pub max_recent_logs: Option<usize>,
    pub max_recent_events: Option<usize>,
    pub max_recent_services: Option<usize>,
    pub max_recent_tests: Option<usize>,
    pub max_changed_files: Option<usize>,
}

impl PartialAiConfig {
    fn apply_to(self, config: &mut AiConfig) {
        if let Some(value) = self.enabled {
            config.enabled = value;
        }
        if self.provider.is_some() {
            config.provider = self.provider;
        }
        if self.model.is_some() {
            config.model = self.model;
        }
        if let Some(value) = self.request_timeout_secs {
            config.request_timeout_secs = value;
        }
        if let Some(value) = self.history_limit {
            config.history_limit = value;
        }
        if let Some(value) = self.max_recent_commands {
            config.max_recent_commands = value;
        }
        if let Some(value) = self.max_recent_logs {
            config.max_recent_logs = value;
        }
        if let Some(value) = self.max_recent_events {
            config.max_recent_events = value;
        }
        if let Some(value) = self.max_recent_services {
            config.max_recent_services = value;
        }
        if let Some(value) = self.max_recent_tests {
            config.max_recent_tests = value;
        }
        if let Some(value) = self.max_changed_files {
            config.max_changed_files = value;
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PartialObservabilityConfig {
    pub log_level: Option<String>,
    pub diagnostics_capacity: Option<usize>,
    pub log_to_file: Option<bool>,
}

impl PartialObservabilityConfig {
    fn apply_to(self, config: &mut ObservabilityConfig) {
        if let Some(value) = self.log_level {
            config.log_level = value;
        }
        if let Some(value) = self.diagnostics_capacity {
            config.diagnostics_capacity = value;
        }
        if let Some(value) = self.log_to_file {
            config.log_to_file = value;
        }
    }
}
