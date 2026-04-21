use std::time::SystemTime;

use crate::config::ForgeConfig;
use crate::domain::{
    AiSessionState, ApprovalState, CommandState, DiagnosticsState, GitSnapshot, JobState, LogState,
    ProcessState, ProjectContext, ServiceState, TestState, TimelineState,
};
use crate::shared::ids::{ApprovalId, NotificationId};
use crate::shared::time::now_utc;

#[derive(Debug, Clone)]
pub struct AppMetaState {
    pub started_at: SystemTime,
    pub tick_count: u64,
    pub should_quit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    CommandPane,
    DashboardPane,
    EventStream,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardTab {
    Services,
    Processes,
    Logs,
    Git,
    Tests,
}

impl DashboardTab {
    pub fn all() -> [DashboardTab; 5] {
        [
            DashboardTab::Services,
            DashboardTab::Processes,
            DashboardTab::Logs,
            DashboardTab::Git,
            DashboardTab::Tests,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            DashboardTab::Services => "Services",
            DashboardTab::Processes => "Processes",
            DashboardTab::Logs => "Logs",
            DashboardTab::Git => "Git",
            DashboardTab::Tests => "Tests",
        }
    }
}

#[derive(Debug, Clone)]
pub enum ModalState {
    Approval(ApprovalId),
    Help,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub focus: FocusTarget,
    pub dashboard_tab: DashboardTab,
    pub input_buffer: String,
    pub modal: Option<ModalState>,
    pub terminal_size: (u16, u16),
    pub approval_cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: NotificationId,
    pub level: NotificationLevel,
    pub message: String,
    pub created_at: SystemTime,
}

#[derive(Debug, Clone, Default)]
pub struct NotificationState {
    pub items: Vec<Notification>,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub app: AppMetaState,
    pub ui: UiState,
    pub config: ForgeConfig,
    pub project: ProjectContext,
    pub git: GitSnapshot,
    pub commands: CommandState,
    pub jobs: JobState,
    pub services: ServiceState,
    pub processes: ProcessState,
    pub logs: LogState,
    pub tests: TestState,
    pub timeline: TimelineState,
    pub approvals: ApprovalState,
    pub ai: AiSessionState,
    pub diagnostics: DiagnosticsState,
    pub notifications: NotificationState,
}

impl AppState {
    pub fn new(config: ForgeConfig, project: ProjectContext, git: GitSnapshot) -> Self {
        let diagnostics_capacity = config.observability.diagnostics_capacity;
        let global_log_capacity = config.logs.global_capacity;
        Self {
            app: AppMetaState {
                started_at: now_utc(),
                tick_count: 0,
                should_quit: false,
            },
            ui: UiState {
                focus: FocusTarget::CommandPane,
                dashboard_tab: DashboardTab::Services,
                input_buffer: String::new(),
                modal: None,
                terminal_size: (0, 0),
                approval_cursor: 0,
            },
            config: config.clone(),
            project,
            git,
            commands: CommandState::default(),
            jobs: JobState::default(),
            services: ServiceState::default(),
            processes: ProcessState::default(),
            logs: LogState::new(global_log_capacity),
            tests: TestState::default(),
            timeline: TimelineState::new(global_log_capacity),
            approvals: ApprovalState::default(),
            ai: AiSessionState::from_config(
                config.ai.enabled,
                config.ai.provider.clone(),
                config.ai.model.clone(),
            ),
            diagnostics: DiagnosticsState::new(diagnostics_capacity),
            notifications: NotificationState::default(),
        }
    }
}
