use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::domain::approval::SafetyClass;
use crate::shared::ids::{CommandId, JobId, ServiceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecutionMode {
    Managed,
    Pty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CommandProvenance {
    UserInput,
    SlashCommand,
    ApprovalEscalation,
    AiSuggestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommandHistoryEntry {
    pub command_id: Option<CommandId>,
    pub raw: String,
    pub cwd: PathBuf,
    pub shell: String,
    pub provenance: CommandProvenance,
    pub background: bool,
    pub safety_class: SafetyClass,
    pub status: CommandStatus,
    pub submitted_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub ended_at: Option<SystemTime>,
    pub exit_code: Option<i32>,
}

impl CommandHistoryEntry {
    pub fn from_request(
        request: &ExecutionRequest,
        status: CommandStatus,
        submitted_at: SystemTime,
    ) -> Self {
        Self {
            command_id: Some(request.command_id),
            raw: request.raw.clone(),
            cwd: request.cwd.clone(),
            shell: request.shell.clone(),
            provenance: request.provenance,
            background: request.background,
            safety_class: request.safety_class,
            status,
            submitted_at,
            started_at: None,
            ended_at: None,
            exit_code: None,
        }
    }

    pub fn builtin(
        raw: String,
        cwd: PathBuf,
        shell: String,
        safety_class: SafetyClass,
        status: CommandStatus,
        exit_code: Option<i32>,
        submitted_at: SystemTime,
    ) -> Self {
        let ended_at = matches!(
            status,
            CommandStatus::Succeeded
                | CommandStatus::Failed
                | CommandStatus::Cancelled
                | CommandStatus::Denied
        )
        .then_some(submitted_at);

        Self {
            command_id: None,
            raw,
            cwd,
            shell,
            provenance: CommandProvenance::UserInput,
            background: false,
            safety_class,
            status,
            submitted_at,
            started_at: Some(submitted_at),
            ended_at,
            exit_code,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct CommandState {
    pub session: ShellSessionState,
    pub history: Vec<CommandHistoryEntry>,
    pub records: Vec<CommandRecord>,
    pub active_command: Option<CommandId>,
}

impl CommandState {
    pub fn new(cwd: PathBuf, shell: String) -> Self {
        Self {
            session: ShellSessionState::new(cwd, shell),
            history: Vec::new(),
            records: Vec::new(),
            active_command: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShellSessionState {
    pub cwd: PathBuf,
    pub previous_cwd: Option<PathBuf>,
    pub shell: String,
    pub env: BTreeMap<String, String>,
    pub last_exit_status: Option<i32>,
}

impl ShellSessionState {
    pub fn new(cwd: PathBuf, shell: String) -> Self {
        Self {
            cwd,
            previous_cwd: None,
            shell,
            env: BTreeMap::new(),
            last_exit_status: None,
        }
    }
}

impl Default for ShellSessionState {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        Self::new(cwd, shell)
    }
}
