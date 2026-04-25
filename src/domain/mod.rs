pub mod ai;
pub mod app;
pub mod approval;
pub mod command;
pub mod diagnostics;
pub mod git;
pub mod job;
pub mod log;
pub mod process;
pub mod project;
pub mod service;
pub mod test;
pub mod timeline;

pub use ai::{
    AiActionProposal, AiCitation, AiCommandContext, AiContextBundle, AiGitContext, AiLogContext,
    AiMessage, AiMessageRole, AiProjectContext, AiRequest, AiRequestKind, AiResponse,
    AiServiceContext, AiSessionState, AiShellSessionContext, AiStatus, AiTestContext,
    AiTimelineContext, AiUiContext,
};
pub use app::{
    AppMetaState, AppState, DashboardTab, FocusTarget, InputEditorState, InputMode, ModalState,
    Notification, NotificationLevel, NotificationState, UiState,
};
pub use approval::{ApprovalDecision, ApprovalMode, ApprovalRequest, ApprovalState, SafetyClass};
pub use command::{
    CommandHistoryEntry, CommandProvenance, CommandRecord, CommandState, CommandStatus,
    ExecutionMode, ExecutionRequest, OutputStream, ShellSessionState,
};
pub use diagnostics::{DiagnosticLevel, DiagnosticRecord, DiagnosticsState};
pub use git::GitSnapshot;
pub use job::{JobRecord, JobState, JobStatus};
pub use log::{LogEntry, LogFilters, LogSeverity, LogSource, LogState, LogStream};
pub use process::{ProcessSnapshot, ProcessState, ProcessStatus};
pub use project::ProjectContext;
pub use service::{ServiceHealth, ServiceRecord, ServiceSource, ServiceState};
pub use test::{TestRunRecord, TestState, TestStatus};
pub use timeline::{TimelineEntry, TimelineKind, TimelineState};
