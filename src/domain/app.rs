use std::time::SystemTime;

use crate::config::ForgeConfig;
use crate::domain::{
    AiRequestKind, AiSessionState, ApprovalState, CommandState, DiagnosticsState, GitSnapshot,
    JobState, LogState, ProcessState, ProjectContext, ServiceState, TestState, TimelineState,
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
    History,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Shell,
    AiAssist,
    AiDiagnose,
}

impl InputMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Shell => "SHELL",
            Self::AiAssist => "AI",
            Self::AiDiagnose => "DIAG",
        }
    }

    pub fn prompt_label(&self) -> &'static str {
        match self {
            Self::Shell => "$",
            Self::AiAssist => "assist>",
            Self::AiDiagnose => "diagnose>",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Shell => Self::AiAssist,
            Self::AiAssist => Self::AiDiagnose,
            Self::AiDiagnose => Self::Shell,
        }
    }

    pub fn ai_kind(self) -> Option<AiRequestKind> {
        match self {
            Self::Shell => None,
            Self::AiAssist => Some(AiRequestKind::Assist),
            Self::AiDiagnose => Some(AiRequestKind::Diagnose),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InputEditorState {
    pub mode: InputMode,
    pub buffer: String,
    pub cursor: usize,
    pub history_cursor: Option<usize>,
    pub history_draft: Option<String>,
    preferred_column: Option<usize>,
}

impl Default for InputEditorState {
    fn default() -> Self {
        Self {
            mode: InputMode::Shell,
            buffer: String::new(),
            cursor: 0,
            history_cursor: None,
            history_draft: None,
            preferred_column: None,
        }
    }
}

impl InputEditorState {
    pub fn is_empty_trimmed(&self) -> bool {
        self.buffer.trim().is_empty()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
        self.reset_history_navigation();
    }

    pub fn take_buffer(&mut self) -> String {
        let raw = std::mem::take(&mut self.buffer);
        self.cursor = 0;
        self.reset_history_navigation();
        raw
    }

    pub fn set_buffer(&mut self, buffer: String) {
        self.buffer = buffer;
        self.cursor = self.buffer.len();
        self.preferred_column = None;
    }

    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
        self.reset_history_navigation();
    }

    pub fn cycle_mode(&mut self) {
        self.mode = self.mode.next();
        self.reset_history_navigation();
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        self.reset_history_navigation();
    }

    pub fn insert_newline(&mut self) {
        self.buffer.insert(self.cursor, '\n');
        self.cursor += 1;
        self.reset_history_navigation();
    }

    pub fn backspace(&mut self) {
        if let Some(previous) = prev_char_boundary(&self.buffer, self.cursor) {
            self.buffer.drain(previous..self.cursor);
            self.cursor = previous;
            self.reset_history_navigation();
        }
    }

    pub fn move_left(&mut self) {
        if let Some(previous) = prev_char_boundary(&self.buffer, self.cursor) {
            self.cursor = previous;
            self.preferred_column = None;
        }
    }

    pub fn move_right(&mut self) {
        if let Some(next) = next_char_boundary(&self.buffer, self.cursor) {
            self.cursor = next;
            self.preferred_column = None;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = line_start(&self.buffer, self.cursor);
        self.preferred_column = None;
    }

    pub fn move_end(&mut self) {
        self.cursor = line_end(&self.buffer, self.cursor);
        self.preferred_column = None;
    }

    pub fn move_up(&mut self) {
        let current_start = line_start(&self.buffer, self.cursor);
        if current_start == 0 {
            self.cursor = 0;
            return;
        }

        let target_column = self
            .preferred_column
            .unwrap_or_else(|| display_column(&self.buffer, self.cursor));
        let previous_end = current_start.saturating_sub(1);
        let previous_start = line_start(&self.buffer, previous_end);
        self.cursor = column_to_offset(&self.buffer, previous_start, previous_end, target_column);
        self.preferred_column = Some(target_column);
    }

    pub fn move_down(&mut self) {
        let current_end = line_end(&self.buffer, self.cursor);
        if current_end >= self.buffer.len() {
            self.cursor = self.buffer.len();
            return;
        }

        let target_column = self
            .preferred_column
            .unwrap_or_else(|| display_column(&self.buffer, self.cursor));
        let next_start = current_end + 1;
        let next_end = line_end(&self.buffer, next_start);
        self.cursor = column_to_offset(&self.buffer, next_start, next_end, target_column);
        self.preferred_column = Some(target_column);
    }

    pub fn line_count(&self) -> usize {
        self.buffer.lines().count().max(1)
    }

    fn reset_history_navigation(&mut self) {
        self.history_cursor = None;
        self.history_draft = None;
        self.preferred_column = None;
    }
}

#[derive(Debug, Clone)]
pub struct UiState {
    pub focus: FocusTarget,
    pub dashboard_tab: DashboardTab,
    pub input: InputEditorState,
    pub command_palette_cursor: usize,
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
        let session_cwd = project.root.clone();
        let session_shell = config.commands.default_shell.clone();
        Self {
            app: AppMetaState {
                started_at: now_utc(),
                tick_count: 0,
                should_quit: false,
            },
            ui: UiState {
                focus: FocusTarget::CommandPane,
                dashboard_tab: DashboardTab::Services,
                input: InputEditorState::default(),
                command_palette_cursor: 0,
                modal: None,
                terminal_size: (0, 0),
                approval_cursor: 0,
            },
            config: config.clone(),
            project,
            git,
            commands: CommandState::new(session_cwd, session_shell),
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

fn prev_char_boundary(value: &str, cursor: usize) -> Option<usize> {
    if cursor == 0 {
        return None;
    }

    let mut previous = 0;
    for (index, _) in value.char_indices() {
        if index >= cursor {
            break;
        }
        previous = index;
    }
    Some(previous)
}

fn next_char_boundary(value: &str, cursor: usize) -> Option<usize> {
    if cursor >= value.len() {
        return None;
    }

    for (index, ch) in value[cursor..].char_indices() {
        if index == 0 {
            return Some(cursor + ch.len_utf8());
        }
    }
    None
}

fn line_start(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn line_end(value: &str, cursor: usize) -> usize {
    value[cursor..]
        .find('\n')
        .map(|index| cursor + index)
        .unwrap_or(value.len())
}

fn display_column(value: &str, cursor: usize) -> usize {
    value[line_start(value, cursor)..cursor].chars().count()
}

fn column_to_offset(value: &str, line_start: usize, line_end: usize, column: usize) -> usize {
    let line = &value[line_start..line_end];
    let mut offset = line_start;
    for (chars, ch) in line.chars().enumerate() {
        if chars >= column {
            break;
        }
        offset += ch.len_utf8();
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::{InputEditorState, InputMode};

    #[test]
    fn input_editor_moves_between_lines() {
        let mut editor = InputEditorState::default();
        editor.set_buffer("alpha\nbeta".to_string());
        editor.cursor = editor.buffer.len();

        editor.move_up();
        assert_eq!(editor.buffer[..editor.cursor].chars().count(), 4);

        editor.move_home();
        assert_eq!(editor.cursor, 0);
    }

    #[test]
    fn input_mode_cycles_across_shell_and_ai_modes() {
        let mut editor = InputEditorState::default();
        assert!(matches!(editor.mode, InputMode::Shell));
        editor.cycle_mode();
        assert!(matches!(editor.mode, InputMode::AiAssist));
        editor.cycle_mode();
        assert!(matches!(editor.mode, InputMode::AiDiagnose));
        editor.cycle_mode();
        assert!(matches!(editor.mode, InputMode::Shell));
    }
}
