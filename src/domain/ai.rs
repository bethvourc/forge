use std::path::PathBuf;
use std::time::SystemTime;

use crate::domain::{
    approval::SafetyClass, command::CommandProvenance, command::CommandStatus, log::LogSeverity,
    log::LogStream, service::ServiceHealth, test::TestStatus, timeline::TimelineKind,
};
use crate::shared::ids::{AiRequestId, CommandId, ServiceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiRequestKind {
    Assist,
    Diagnose,
}

impl AiRequestKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Assist => "Assist",
            Self::Diagnose => "Diagnose",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiStatus {
    Disabled,
    Unconfigured,
    Ready,
    Queued,
    Running,
    Completed,
    Failed,
}

impl AiStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Disabled => "OFF",
            Self::Unconfigured => "SETUP",
            Self::Ready => "READY",
            Self::Queued => "QUEUED",
            Self::Running => "RUNNING",
            Self::Completed => "DONE",
            Self::Failed => "ERROR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct AiMessage {
    pub role: AiMessageRole,
    pub content: String,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct AiUiContext {
    pub focus: String,
    pub dashboard_tab: String,
    pub modal: Option<String>,
    pub pending_approvals: usize,
}

#[derive(Debug, Clone)]
pub struct AiProjectContext {
    pub name: String,
    pub root: PathBuf,
    pub stack_hints: Vec<String>,
    pub config_paths: Vec<PathBuf>,
    pub discovered_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AiGitContext {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub is_dirty: bool,
    pub changed_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AiShellSessionContext {
    pub cwd: PathBuf,
    pub shell: String,
    pub env_overlay_count: usize,
    pub last_exit_status: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct AiCommandContext {
    pub id: CommandId,
    pub raw: String,
    pub cwd: PathBuf,
    pub provenance: CommandProvenance,
    pub status: CommandStatus,
    pub exit_code: Option<i32>,
    pub output_line_count: usize,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub struct AiServiceContext {
    pub id: ServiceId,
    pub name: String,
    pub health: ServiceHealth,
    pub pid: Option<u32>,
    pub ports: Vec<u16>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AiTestContext {
    pub runner: String,
    pub status: TestStatus,
    pub pass_count: usize,
    pub fail_count: usize,
    pub failed_tests: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AiLogContext {
    pub ts: SystemTime,
    pub source_label: String,
    pub stream: LogStream,
    pub severity: LogSeverity,
    pub service_id: Option<ServiceId>,
    pub command_id: Option<CommandId>,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct AiTimelineContext {
    pub at: SystemTime,
    pub kind: TimelineKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AiContextBundle {
    pub ui: AiUiContext,
    pub project: AiProjectContext,
    pub git: AiGitContext,
    pub session: AiShellSessionContext,
    pub commands: Vec<AiCommandContext>,
    pub services: Vec<AiServiceContext>,
    pub tests: Vec<AiTestContext>,
    pub logs: Vec<AiLogContext>,
    pub timeline: Vec<AiTimelineContext>,
}

#[derive(Debug, Clone)]
pub struct AiRequest {
    pub id: AiRequestId,
    pub kind: AiRequestKind,
    pub created_at: SystemTime,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub context: AiContextBundle,
}

#[derive(Debug, Clone)]
pub struct AiCitation {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct AiActionProposal {
    pub summary: String,
    pub detail: String,
    pub command: Option<String>,
    pub safety_class: SafetyClass,
}

#[derive(Debug, Clone)]
pub struct AiResponse {
    pub request_id: AiRequestId,
    pub created_at: SystemTime,
    pub provider: String,
    pub model: Option<String>,
    pub summary: String,
    pub message: String,
    pub recommendations: Vec<String>,
    pub proposals: Vec<AiActionProposal>,
    pub citations: Vec<AiCitation>,
}

#[derive(Debug, Clone)]
pub struct AiSessionState {
    pub enabled: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: AiStatus,
    pub active_request: Option<AiRequestId>,
    pub requests: Vec<AiRequest>,
    pub messages: Vec<AiMessage>,
    pub last_context: Option<AiContextBundle>,
    pub last_response: Option<AiResponse>,
    pub last_error: Option<String>,
}

impl AiSessionState {
    pub fn from_config(enabled: bool, provider: Option<String>, model: Option<String>) -> Self {
        let status = if !enabled {
            AiStatus::Disabled
        } else if provider.is_some() {
            AiStatus::Ready
        } else {
            AiStatus::Unconfigured
        };

        Self {
            enabled,
            provider,
            model,
            status,
            active_request: None,
            requests: Vec::new(),
            messages: Vec::new(),
            last_context: None,
            last_response: None,
            last_error: None,
        }
    }

    pub fn push_request(&mut self, request: AiRequest, limit: usize) {
        self.active_request = Some(request.id);
        self.last_context = Some(request.context.clone());
        self.requests.push(request);
        trim_vec(&mut self.requests, limit);
    }

    pub fn push_message(&mut self, message: AiMessage, limit: usize) {
        self.messages.push(message);
        trim_vec(&mut self.messages, limit);
    }
}

fn trim_vec<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        let overflow = items.len() - limit;
        items.drain(0..overflow);
    }
}
