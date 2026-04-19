use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::domain::approval::SafetyClass;
use crate::shared::ids::{CommandId, JobId, ServiceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Managed,
    Pty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandProvenance {
    UserInput,
    SlashCommand,
    ApprovalEscalation,
    AiSuggestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    PendingReview,
    PendingApproval,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Denied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub command_id: CommandId,
    pub job_id: JobId,
    pub service_id: Option<ServiceId>,
    pub raw: String,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub shell: String,
    pub mode: ExecutionMode,
    pub provenance: CommandProvenance,
    pub background: bool,
    pub safety_class: SafetyClass,
}

#[derive(Debug, Clone)]
pub struct CommandRecord {
    pub id: CommandId,
    pub raw: String,
    pub cwd: PathBuf,
    pub provenance: CommandProvenance,
    pub safety_class: SafetyClass,
    pub background: bool,
    pub status: CommandStatus,
    pub started_at: Option<SystemTime>,
    pub ended_at: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub output_line_count: usize,
}

impl CommandRecord {
    pub fn from_request(request: &ExecutionRequest, status: CommandStatus) -> Self {
        Self {
            id: request.command_id,
            raw: request.raw.clone(),
            cwd: request.cwd.clone(),
            provenance: request.provenance,
            safety_class: request.safety_class,
            background: request.background,
            status,
            started_at: None,
            ended_at: None,
            exit_code: None,
            output_line_count: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandState {
    pub history: Vec<String>,
    pub records: Vec<CommandRecord>,
    pub active_command: Option<CommandId>,
}
