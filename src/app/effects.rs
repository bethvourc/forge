use std::path::PathBuf;

use crate::domain::{
    ApprovalRequest, DiagnosticLevel, GitSnapshot, OutputStream, ProjectContext,
};
use crate::domain::ExecutionRequest;
use crate::shared::ids::{ApprovalId, CommandId, JobId, ServiceId};

#[derive(Debug, Clone)]
pub enum UiIntent {
    KeyChar(char),
    Backspace,
    Submit,
    ClearInput,
    NextFocus,
    PrevFocus,
    NextTab,
    PrevTab,
    ApproveCurrent,
    DenyCurrent,
    Resize(u16, u16),
    Tick,
    Quit,
    Esc,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    Ui(UiIntent),
    ParsedInput(crate::commands::ParsedInput),
    Approve(ApprovalId),
    Deny(ApprovalId),
    Cancel(CommandId),
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    ProjectScanned(ProjectContext),
    GitRefreshed(GitSnapshot),
    CommandStarted {
        command_id: CommandId,
        job_id: JobId,
        service_id: Option<ServiceId>,
        pid: Option<u32>,
    },
    CommandOutput {
        command_id: CommandId,
        job_id: JobId,
        service_id: Option<ServiceId>,
        stream: OutputStream,
        chunk: String,
    },
    CommandExited {
        command_id: CommandId,
        job_id: JobId,
        service_id: Option<ServiceId>,
        exit_code: i32,
    },
    CommandCancelled {
        command_id: CommandId,
        job_id: JobId,
        service_id: Option<ServiceId>,
    },
    Diagnostic {
        level: DiagnosticLevel,
        message: String,
        context: Option<String>,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum Effect {
    ExecuteCommand(ExecutionRequest),
    CancelCommand(CommandId),
    RefreshProjectContext(PathBuf),
    RefreshGit(PathBuf),
    Shutdown,
    QueueApproval(ApprovalRequest),
}
