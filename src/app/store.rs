use std::path::{Path, PathBuf};

use crate::ai;
use crate::app::{AppAction, AppEvent, Effect, UiIntent};
use crate::commands::{parse_input, parse_shell_input, slash_command_suggestions, ParsedInput};
use crate::domain::{
    AiActionProposal, AiMessage, AiMessageRole, AiRequest, AiRequestKind, AiStatus, AppState,
    ApprovalDecision, ApprovalMode, ApprovalRequest, CommandHistoryEntry, CommandRecord,
    CommandStatus, DashboardTab, DiagnosticLevel, DiagnosticRecord, ExecutionMode,
    ExecutionRequest, FocusTarget, InputMode, JobRecord, JobStatus, ModalState, Notification,
    NotificationLevel, ProcessSnapshot, ProcessStatus, ServiceHealth, ServiceRecord, ServiceSource,
    TimelineEntry, TimelineKind,
};
use crate::safety::{approval_message, approval_mode_for, classify_command};
use crate::shared::ids::{ApprovalId, CommandId, IdGenerator};
use crate::shared::time::now_utc;

pub struct AppStore {
    state: AppState,
    ids: IdGenerator,
}

impl AppStore {
    pub fn new(state: AppState) -> Self {
        let next_id = next_id_after_restored_state(&state);
        Self {
            state,
            ids: IdGenerator::starting_at(next_id),
        }
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    pub fn dispatch_action(&mut self, action: AppAction) -> Vec<Effect> {
        match action {
            AppAction::Ui(intent) => self.handle_ui_intent(intent),
            AppAction::ParsedInput(input) => self.handle_parsed_input(input),
            AppAction::Approve(id) => self.resolve_approval(id, true),
            AppAction::Deny(id) => self.resolve_approval(id, false),
            AppAction::Cancel(command_id) => {
                self.append_timeline(
                    TimelineKind::Command,
                    format!("cancellation requested for command #{command_id}"),
                );
                vec![Effect::CancelCommand(command_id)]
            }
        }
    }

    pub fn dispatch_event(&mut self, event: AppEvent) -> Vec<Effect> {
        match event {
            AppEvent::ProjectScanned(project) => {
                self.state.project = project.clone();
                self.append_timeline(
                    TimelineKind::System,
                    format!("project context refreshed for {}", project.name),
                );
            }
            AppEvent::GitRefreshed(snapshot) => {
                self.state.git = snapshot;
            }
            AppEvent::AiStarted {
                request_id,
                provider,
            } => {
                self.state.ai.status = AiStatus::Running;
                self.state.ai.active_request = Some(request_id);
                self.append_timeline(
                    TimelineKind::Ai,
                    format!("AI request #{request_id} started via {provider}"),
                );
            }
            AppEvent::AiCompleted(response) => {
                let request_id = response.request_id;
                let summary = truncate_for_timeline(&response.summary, 96);
                let proposal_count = response.proposals.len();
                let history_limit = self.state.config.ai.history_limit;
                self.state.ai.status = AiStatus::Completed;
                self.state.ai.active_request = None;
                self.state.ai.last_error = None;
                self.state.ai.last_response = Some(response.clone());
                self.state.ai.push_message(
                    AiMessage {
                        role: AiMessageRole::Assistant,
                        content: response.message.clone(),
                        created_at: now_utc(),
                    },
                    history_limit,
                );
                self.append_timeline(
                    TimelineKind::Ai,
                    format!(
                        "AI response ready for request #{request_id}: {summary}{}",
                        if proposal_count > 0 {
                            format!(" ({proposal_count} proposal(s))")
                        } else {
                            String::new()
                        }
                    ),
                );
                self.push_notification(
                    NotificationLevel::Info,
                    if proposal_count > 0 {
                        format!(
                            "AI response ready for request #{request_id} with {proposal_count} proposal(s)"
                        )
                    } else {
                        format!("AI response ready for request #{request_id}")
                    },
                );
            }
            AppEvent::AiFailed {
                request_id,
                provider,
                message,
            } => {
                self.state.ai.status = AiStatus::Failed;
                self.state.ai.active_request = None;
                self.state.ai.last_error = Some(message.clone());
                self.append_timeline(
                    TimelineKind::Ai,
                    format!(
                        "AI request #{request_id} failed via {provider}: {}",
                        truncate_for_timeline(&message, 96)
                    ),
                );
                self.push_notification(
                    NotificationLevel::Warn,
                    format!("AI request #{request_id} failed: {message}"),
                );
            }
            AppEvent::CommandStarted {
                command_id,
                job_id,
                service_id,
                pid,
            } => {
                let started_at = now_utc();
                if let Some(command) = self.find_command_mut(command_id) {
                    command.status = CommandStatus::Running;
                    command.started_at = Some(started_at);
                }
                self.update_history_for_command(command_id, |entry| {
                    entry.status = CommandStatus::Running;
                    entry.started_at = Some(started_at);
                });
                if let Some(job) = self.find_job_mut(job_id) {
                    job.status = JobStatus::Running;
                    job.started_at = Some(started_at);
                    job.pid = pid;
                }
                let process = ProcessSnapshot {
                    id: self.ids.next_process(),
                    command_id: Some(command_id),
                    job_id: Some(job_id),
                    pid,
                    label: self
                        .find_command(command_id)
                        .map(|command| command.raw.clone())
                        .unwrap_or_else(|| "managed command".to_string()),
                    status: ProcessStatus::Running,
                    cpu_percent: None,
                    memory_bytes: None,
                    observed_at: now_utc(),
                };
                self.upsert_process(process);
                if let Some(service_id) = service_id {
                    self.upsert_service(ServiceRecord {
                        id: service_id,
                        name: self
                            .find_command(command_id)
                            .map(|command| command.raw.clone())
                            .unwrap_or_else(|| "background service".to_string()),
                        source: ServiceSource::ManagedCommand,
                        pid,
                        ports: Vec::new(),
                        health: ServiceHealth::Healthy,
                        tags: vec!["managed".to_string()],
                        linked_command: Some(command_id),
                        last_seen: now_utc(),
                    });
                }
                self.state.commands.active_command = Some(command_id);
                self.append_timeline(
                    TimelineKind::Command,
                    format!(
                        "command #{command_id} started{}",
                        pid.map(|pid| format!(" (pid {pid})")).unwrap_or_default()
                    ),
                );
            }
            AppEvent::CommandOutput {
                command_id,
                job_id: _,
                service_id,
                stream,
                chunk,
            } => {
                if let Some(command) = self.find_command_mut(command_id) {
                    command.output_line_count += 1;
                }
                let log_entry = crate::infra::logs::normalize_command_output(
                    self.ids.next_log(),
                    now_utc(),
                    command_id,
                    service_id,
                    stream,
                    chunk.clone(),
                );
                let source_key = crate::infra::logs::source_key(&log_entry.source);
                let per_source_capacity = self.state.config.logs.per_source_capacity;
                self.state.logs.recent.push(log_entry.clone());
                self.state
                    .logs
                    .per_source
                    .entry(source_key)
                    .or_insert_with(|| {
                        crate::shared::ring_buffer::RingBuffer::new(per_source_capacity)
                    })
                    .push(log_entry.clone());
                self.append_timeline(
                    TimelineKind::Log,
                    format!(
                        "#{} {}",
                        command_id,
                        truncate_for_timeline(&log_entry.raw, 96)
                    ),
                );
            }
            AppEvent::CommandExited {
                command_id,
                job_id,
                service_id,
                exit_code,
            } => {
                let succeeded = exit_code == 0;
                let ended_at = now_utc();
                if let Some(command) = self.find_command_mut(command_id) {
                    command.status = if succeeded {
                        CommandStatus::Succeeded
                    } else {
                        CommandStatus::Failed
                    };
                    command.exit_code = Some(exit_code);
                    command.ended_at = Some(ended_at);
                }
                self.update_history_for_command(command_id, |entry| {
                    entry.status = if succeeded {
                        CommandStatus::Succeeded
                    } else {
                        CommandStatus::Failed
                    };
                    entry.exit_code = Some(exit_code);
                    entry.ended_at = Some(ended_at);
                });
                self.state.commands.session.last_exit_status = Some(exit_code);
                if let Some(job) = self.find_job_mut(job_id) {
                    job.status = if succeeded {
                        JobStatus::Succeeded
                    } else {
                        JobStatus::Failed
                    };
                    job.ended_at = Some(ended_at);
                }
                if let Some(service_id) = service_id {
                    if let Some(service) = self.find_service_mut(service_id) {
                        service.health = if succeeded {
                            ServiceHealth::Stopped
                        } else {
                            ServiceHealth::Degraded
                        };
                        service.last_seen = now_utc();
                    }
                }
                self.mark_process(
                    command_id,
                    if succeeded {
                        ProcessStatus::Exited
                    } else {
                        ProcessStatus::Failed
                    },
                );
                self.state.commands.active_command = None;
                self.append_timeline(
                    TimelineKind::Command,
                    format!("command #{command_id} exited with code {exit_code}"),
                );
                if !succeeded {
                    self.push_notification(
                        NotificationLevel::Warn,
                        format!("command #{command_id} failed with exit code {exit_code}"),
                    );
                }
            }
            AppEvent::CommandCancelled {
                command_id,
                job_id,
                service_id,
            } => {
                let ended_at = now_utc();
                if let Some(command) = self.find_command_mut(command_id) {
                    command.status = CommandStatus::Cancelled;
                    command.ended_at = Some(ended_at);
                }
                self.update_history_for_command(command_id, |entry| {
                    entry.status = CommandStatus::Cancelled;
                    entry.exit_code = Some(130);
                    entry.ended_at = Some(ended_at);
                });
                self.state.commands.session.last_exit_status = Some(130);
                if let Some(job) = self.find_job_mut(job_id) {
                    job.status = JobStatus::Cancelled;
                    job.ended_at = Some(ended_at);
                }
                if let Some(service_id) = service_id {
                    if let Some(service) = self.find_service_mut(service_id) {
                        service.health = ServiceHealth::Stopped;
                        service.last_seen = now_utc();
                    }
                }
                self.mark_process(command_id, ProcessStatus::Cancelled);
                self.append_timeline(
                    TimelineKind::Command,
                    format!("command #{command_id} cancelled"),
                );
            }
            AppEvent::Diagnostic {
                level,
                message,
                context,
            } => {
                self.state.diagnostics.records.push(DiagnosticRecord {
                    at: now_utc(),
                    level,
                    message: message.clone(),
                    context,
                });
                if matches!(level, DiagnosticLevel::Warn | DiagnosticLevel::Error) {
                    self.push_notification(NotificationLevel::Warn, message.clone());
                }
            }
            AppEvent::Error(message) => {
                self.state.ui.modal = Some(ModalState::Error(message.clone()));
                self.state.ui.focus = FocusTarget::Modal;
                self.state.diagnostics.records.push(DiagnosticRecord {
                    at: now_utc(),
                    level: DiagnosticLevel::Error,
                    message: message.clone(),
                    context: None,
                });
                self.append_timeline(TimelineKind::Error, message.clone());
                self.push_notification(NotificationLevel::Error, message);
            }
        }
        Vec::new()
    }

    fn handle_ui_intent(&mut self, intent: UiIntent) -> Vec<Effect> {
        match intent {
            UiIntent::KeyChar(c) => {
                if let Some(ModalState::Approval(_)) = self.state.ui.modal.as_ref() {
                    match c {
                        'y' | 'Y' => {
                            return self.dispatch_action(AppAction::Ui(UiIntent::ApproveCurrent))
                        }
                        'n' | 'N' => {
                            return self.dispatch_action(AppAction::Ui(UiIntent::DenyCurrent))
                        }
                        _ => return Vec::new(),
                    }
                } else if matches!(self.state.ui.focus, FocusTarget::CommandPane) {
                    self.state.ui.input.insert_char(c);
                    self.sync_command_palette_cursor();
                } else if self.state.ui.modal.is_some() {
                    return Vec::new();
                }
            }
            UiIntent::Backspace => {
                self.state.ui.input.backspace();
                self.sync_command_palette_cursor();
            }
            UiIntent::InsertNewline => {
                if matches!(self.state.ui.focus, FocusTarget::CommandPane)
                    && self.state.ui.modal.is_none()
                {
                    self.state.ui.input.insert_newline();
                    self.sync_command_palette_cursor();
                }
            }
            UiIntent::Submit => {
                if let Some(modal) = self.state.ui.modal.as_ref() {
                    return match modal {
                        ModalState::Approval(_) => {
                            self.dispatch_action(AppAction::Ui(UiIntent::ApproveCurrent))
                        }
                        ModalState::Help | ModalState::History | ModalState::Error(_) => {
                            self.close_modal();
                            Vec::new()
                        }
                    };
                }

                if self.state.ui.input.is_empty_trimmed() {
                    if let Some(approval_id) = self.current_inline_approval_id() {
                        return self.dispatch_action(AppAction::Approve(approval_id));
                    }
                    return Vec::new();
                }

                let raw = self.state.ui.input.take_buffer();
                self.state.ui.command_palette_cursor = 0;
                let parsed = if raw.trim_start().starts_with('/') {
                    parse_input(&raw)
                } else if let Some(kind) = self.state.ui.input.mode.ai_kind() {
                    Ok(ParsedInput::AiPrompt {
                        prompt: raw.trim().to_string(),
                        kind,
                    })
                } else {
                    parse_input(&raw)
                };
                match parsed {
                    Ok(parsed) => return self.dispatch_action(AppAction::ParsedInput(parsed)),
                    Err(message) => {
                        self.push_notification(NotificationLevel::Warn, message.clone());
                        self.append_timeline(TimelineKind::Error, message);
                    }
                }
            }
            UiIntent::ClearInput => {
                self.state.ui.input.clear();
                self.state.ui.command_palette_cursor = 0;
            }
            UiIntent::MoveCursorLeft => {
                self.state.ui.input.move_left();
                self.sync_command_palette_cursor();
            }
            UiIntent::MoveCursorRight => {
                self.state.ui.input.move_right();
                self.sync_command_palette_cursor();
            }
            UiIntent::MoveCursorUp => {
                self.state.ui.input.move_up();
                self.sync_command_palette_cursor();
            }
            UiIntent::MoveCursorDown => {
                self.state.ui.input.move_down();
                self.sync_command_palette_cursor();
            }
            UiIntent::MoveCursorHome => {
                self.state.ui.input.move_home();
                self.sync_command_palette_cursor();
            }
            UiIntent::MoveCursorEnd => {
                self.state.ui.input.move_end();
                self.sync_command_palette_cursor();
            }
            UiIntent::RecallPreviousHistory => {
                self.recall_history(true);
                self.sync_command_palette_cursor();
            }
            UiIntent::RecallNextHistory => {
                self.recall_history(false);
                self.sync_command_palette_cursor();
            }
            UiIntent::CycleInputMode => {
                self.state.ui.input.cycle_mode();
                self.push_notification(
                    NotificationLevel::Info,
                    format!(
                        "input mode: {}",
                        self.state.ui.input.mode.label().to_ascii_lowercase()
                    ),
                );
            }
            UiIntent::PrevCommandSuggestion => {
                self.move_command_palette_cursor(false);
            }
            UiIntent::NextCommandSuggestion => {
                self.move_command_palette_cursor(true);
            }
            UiIntent::AcceptCommandSuggestion => {
                self.accept_command_suggestion();
            }
            UiIntent::NextFocus => {
                if self.state.ui.modal.is_some() {
                    self.state.ui.focus = FocusTarget::Modal;
                    return Vec::new();
                }
                self.state.ui.focus = match self.state.ui.focus {
                    FocusTarget::CommandPane => FocusTarget::DashboardPane,
                    FocusTarget::DashboardPane => FocusTarget::EventStream,
                    FocusTarget::EventStream | FocusTarget::Modal => FocusTarget::CommandPane,
                };
            }
            UiIntent::PrevFocus => {
                if self.state.ui.modal.is_some() {
                    self.state.ui.focus = FocusTarget::Modal;
                    return Vec::new();
                }
                self.state.ui.focus = match self.state.ui.focus {
                    FocusTarget::CommandPane | FocusTarget::Modal => FocusTarget::EventStream,
                    FocusTarget::DashboardPane => FocusTarget::CommandPane,
                    FocusTarget::EventStream => FocusTarget::DashboardPane,
                };
            }
            UiIntent::NextTab => self.advance_tab(true),
            UiIntent::PrevTab => self.advance_tab(false),
            UiIntent::ApproveCurrent => {
                if let Some(approval_id) = self.current_approval_id() {
                    return self.dispatch_action(AppAction::Approve(approval_id));
                } else if self.state.ui.modal.is_some() {
                    self.close_modal();
                }
            }
            UiIntent::DenyCurrent => {
                if let Some(approval_id) = self.current_approval_id() {
                    return self.dispatch_action(AppAction::Deny(approval_id));
                }
                self.close_modal();
            }
            UiIntent::Resize(width, height) => {
                self.state.ui.terminal_size = (width, height);
            }
            UiIntent::Tick => {
                self.state.app.tick_count += 1;
                let mut effects = Vec::new();
                if self.state.config.git.enabled && self.state.app.tick_count.is_multiple_of(50) {
                    effects.push(Effect::RefreshGit(self.state.project.root.clone()));
                }
                if self.state.app.tick_count.is_multiple_of(100) {
                    effects.push(Effect::RefreshProjectContext(
                        self.state.project.root.clone(),
                    ));
                }
                return effects;
            }
            UiIntent::Quit => {
                self.state.app.should_quit = true;
                return vec![Effect::Shutdown];
            }
            UiIntent::Esc => {
                if self.state.ui.modal.is_some() {
                    return match self.state.ui.modal.as_ref() {
                        Some(ModalState::Approval(_)) => {
                            self.dispatch_action(AppAction::Ui(UiIntent::DenyCurrent))
                        }
                        Some(ModalState::Help)
                        | Some(ModalState::History)
                        | Some(ModalState::Error(_)) => {
                            self.close_modal();
                            Vec::new()
                        }
                        None => Vec::new(),
                    };
                }
                self.state.ui.input.clear();
                self.state.ui.command_palette_cursor = 0;
            }
        }
        Vec::new()
    }

    fn handle_parsed_input(&mut self, input: ParsedInput) -> Vec<Effect> {
        match input {
            ParsedInput::Execute {
                command,
                background,
                provenance,
            } => self.prepare_execution(command, background, provenance),
            ParsedInput::ChangeDirectory { target } => self.change_directory(target),
            ParsedInput::Quit => self.dispatch_action(AppAction::Ui(UiIntent::Quit)),
            ParsedInput::Clear => {
                self.state.ui.input.clear();
                self.state.logs.recent.clear();
                self.state.logs.per_source.clear();
                self.append_timeline(
                    TimelineKind::System,
                    "cleared command input and visible log buffer".to_string(),
                );
                Vec::new()
            }
            ParsedInput::NextTab => {
                self.advance_tab(true);
                Vec::new()
            }
            ParsedInput::PrevTab => {
                self.advance_tab(false);
                Vec::new()
            }
            ParsedInput::ApprovePending => {
                if let Some(approval_id) = self.current_approval_id() {
                    self.dispatch_action(AppAction::Approve(approval_id))
                } else {
                    self.push_notification(
                        NotificationLevel::Info,
                        "no pending approvals".to_string(),
                    );
                    Vec::new()
                }
            }
            ParsedInput::DenyPending => {
                if let Some(approval_id) = self.current_approval_id() {
                    self.dispatch_action(AppAction::Deny(approval_id))
                } else {
                    self.push_notification(
                        NotificationLevel::Info,
                        "no pending approvals".to_string(),
                    );
                    Vec::new()
                }
            }
            ParsedInput::Cancel(command_id) => self.dispatch_action(AppAction::Cancel(command_id)),
            ParsedInput::History => {
                self.state.ui.modal = Some(ModalState::History);
                self.state.ui.focus = FocusTarget::Modal;
                self.append_timeline(TimelineKind::System, "opened command history".to_string());
                Vec::new()
            }
            ParsedInput::Rerun(command_id) => self.rerun_command(command_id),
            ParsedInput::RerunLast => self.rerun_last_command(),
            ParsedInput::Help => {
                self.state.ui.modal = Some(ModalState::Help);
                self.state.ui.focus = FocusTarget::Modal;
                self.append_timeline(TimelineKind::System, "opened help reference".to_string());
                Vec::new()
            }
            ParsedInput::AiPrompt { prompt, kind } => self.prepare_ai_request(prompt, kind),
            ParsedInput::ApplyAiProposal(index) => self.apply_ai_proposal(index),
        }
    }

    fn active_suggestion_len(&self) -> usize {
        let slash = slash_command_suggestions(&self.state.ui.input.buffer).len();
        if slash > 0 {
            return slash;
        }
        crate::ui::views::intent_suggestions(&self.state).len()
    }

    fn sync_command_palette_cursor(&mut self) {
        let len = self.active_suggestion_len();
        if len == 0 {
            self.state.ui.command_palette_cursor = 0;
            return;
        }

        if self.state.ui.command_palette_cursor >= len {
            self.state.ui.command_palette_cursor = len.saturating_sub(1);
        }
    }

    fn move_command_palette_cursor(&mut self, forward: bool) {
        let len = self.active_suggestion_len();
        if len == 0 {
            self.state.ui.command_palette_cursor = 0;
            return;
        }
        let current = self
            .state
            .ui
            .command_palette_cursor
            .min(len.saturating_sub(1));
        self.state.ui.command_palette_cursor = if forward {
            (current + 1) % len
        } else if current == 0 {
            len - 1
        } else {
            current - 1
        };
    }

    fn accept_command_suggestion(&mut self) {
        let slash = slash_command_suggestions(&self.state.ui.input.buffer);
        if !slash.is_empty() {
            let index = self
                .state
                .ui
                .command_palette_cursor
                .min(slash.len().saturating_sub(1));
            let suggestion = slash[index];
            self.state
                .ui
                .input
                .set_buffer(suggestion.completion.to_string());
            if !suggestion.accepts_arguments() {
                self.state.ui.command_palette_cursor = index;
            } else {
                self.state.ui.command_palette_cursor = 0;
            }
            return;
        }

        let intents = crate::ui::views::intent_suggestions(&self.state);
        if intents.is_empty() {
            return;
        }
        let index = self
            .state
            .ui
            .command_palette_cursor
            .min(intents.len().saturating_sub(1));
        let completion = intents[index].cmd.clone();
        self.state.ui.input.set_buffer(completion);
        self.state.ui.command_palette_cursor = 0;
    }

    fn recall_history(&mut self, previous: bool) {
        let entries = self.history_entries_for_mode();
        if entries.is_empty() {
            return;
        }

        let editor = &mut self.state.ui.input;
        if previous {
            let next_index = match editor.history_cursor {
                Some(index) if index > 0 => index - 1,
                Some(index) => index,
                None => {
                    editor.history_draft = Some(editor.buffer.clone());
                    entries.len().saturating_sub(1)
                }
            };
            editor.history_cursor = Some(next_index);
            editor.set_buffer(entries[next_index].clone());
            editor.history_cursor = Some(next_index);
            return;
        }

        let Some(current_index) = editor.history_cursor else {
            return;
        };

        if current_index + 1 < entries.len() {
            let next_index = current_index + 1;
            editor.history_cursor = Some(next_index);
            editor.set_buffer(entries[next_index].clone());
            editor.history_cursor = Some(next_index);
            return;
        }

        let draft = editor.history_draft.take().unwrap_or_default();
        editor.set_buffer(draft);
        editor.history_cursor = None;
    }

    fn history_entries_for_mode(&self) -> Vec<String> {
        match self.state.ui.input.mode {
            InputMode::Shell => self
                .state
                .commands
                .history
                .iter()
                .map(|entry| entry.raw.clone())
                .collect(),
            InputMode::AiAssist => self
                .state
                .ai
                .requests
                .iter()
                .filter(|request| matches!(request.kind, AiRequestKind::Assist))
                .map(|request| request.prompt.clone())
                .collect(),
            InputMode::AiDiagnose => self
                .state
                .ai
                .requests
                .iter()
                .filter(|request| matches!(request.kind, AiRequestKind::Diagnose))
                .map(|request| request.prompt.clone())
                .collect(),
        }
    }

    fn rerun_command(&mut self, command_id: CommandId) -> Vec<Effect> {
        let entry = self
            .state
            .commands
            .history
            .iter()
            .rev()
            .find(|entry| entry.command_id == Some(command_id))
            .cloned();

        let Some(entry) = entry else {
            self.push_notification(
                NotificationLevel::Warn,
                format!("no command history entry found for #{command_id}"),
            );
            self.append_timeline(
                TimelineKind::Error,
                format!("rerun requested for missing command #{command_id}"),
            );
            return Vec::new();
        };

        self.replay_history_entry(entry, format!("command #{command_id}"))
    }

    fn rerun_last_command(&mut self) -> Vec<Effect> {
        let Some(entry) = self.state.commands.history.last().cloned() else {
            self.push_notification(
                NotificationLevel::Info,
                "no shell history is available to rerun".to_string(),
            );
            return Vec::new();
        };

        self.replay_history_entry(entry, "latest history entry".to_string())
    }

    fn replay_history_entry(&mut self, entry: CommandHistoryEntry, source: String) -> Vec<Effect> {
        self.append_timeline(
            TimelineKind::Command,
            format!(
                "replaying {source}: {}",
                truncate_for_timeline(&entry.raw, 96)
            ),
        );

        let parsed = match parse_input(&entry.raw) {
            Ok(parsed) => parsed,
            Err(message) => {
                self.push_notification(NotificationLevel::Warn, message.clone());
                self.append_timeline(TimelineKind::Error, message);
                return Vec::new();
            }
        };

        match parsed {
            ParsedInput::Execute { command, .. } => self.prepare_execution_internal(
                command,
                entry.background,
                crate::domain::CommandProvenance::SlashCommand,
                None,
                Some(entry.cwd),
                Some(entry.shell),
            ),
            ParsedInput::ChangeDirectory { target } => self.change_directory(target),
            ParsedInput::Clear => self.handle_parsed_input(ParsedInput::Clear),
            ParsedInput::Quit => {
                self.push_notification(
                    NotificationLevel::Warn,
                    "refusing to replay a quit command".to_string(),
                );
                Vec::new()
            }
            _ => {
                self.push_notification(
                    NotificationLevel::Warn,
                    format!("history entry is not replayable: {}", entry.raw),
                );
                Vec::new()
            }
        }
    }

    fn change_directory(&mut self, target: Option<String>) -> Vec<Effect> {
        let submitted_at = now_utc();
        let current = self.state.commands.session.cwd.clone();
        let previous = self.state.commands.session.previous_cwd.clone();
        match resolve_cd_target(&current, previous.as_deref(), target.as_deref()) {
            Ok(next_cwd) => {
                self.push_shell_history(CommandHistoryEntry::builtin(
                    cd_history_entry(target.as_deref()),
                    current.clone(),
                    self.state.commands.session.shell.clone(),
                    crate::domain::SafetyClass::Passive,
                    CommandStatus::Succeeded,
                    Some(0),
                    submitted_at,
                ));
                self.state.commands.session.previous_cwd = Some(current);
                self.state.commands.session.cwd = next_cwd.clone();
                self.state.commands.session.last_exit_status = Some(0);
                self.append_timeline(
                    TimelineKind::Command,
                    format!("session cwd changed to {}", next_cwd.display()),
                );
                self.push_notification(
                    NotificationLevel::Info,
                    format!("cwd: {}", next_cwd.display()),
                );
            }
            Err(message) => {
                self.push_shell_history(CommandHistoryEntry::builtin(
                    cd_history_entry(target.as_deref()),
                    current.clone(),
                    self.state.commands.session.shell.clone(),
                    crate::domain::SafetyClass::Passive,
                    CommandStatus::Failed,
                    Some(1),
                    submitted_at,
                ));
                self.state.commands.session.last_exit_status = Some(1);
                self.state.diagnostics.records.push(DiagnosticRecord {
                    at: now_utc(),
                    level: DiagnosticLevel::Warn,
                    message: message.clone(),
                    context: Some(format!("cwd: {}", current.display())),
                });
                self.append_timeline(TimelineKind::Error, message.clone());
                self.push_notification(NotificationLevel::Warn, message);
            }
        }

        Vec::new()
    }

    fn prepare_ai_request(&mut self, prompt: String, kind: AiRequestKind) -> Vec<Effect> {
        if !self.state.ai.enabled {
            let message = "AI is disabled in the current Forge configuration.".to_string();
            self.state.ai.status = AiStatus::Disabled;
            self.state.ai.last_error = Some(message.clone());
            self.push_notification(NotificationLevel::Warn, message.clone());
            self.append_timeline(TimelineKind::Ai, message);
            return Vec::new();
        }

        let request = AiRequest {
            id: self.ids.next_ai_request(),
            kind,
            prompt: prompt.clone(),
            created_at: now_utc(),
            provider: self.state.ai.provider.clone(),
            model: self.state.ai.model.clone(),
            context: ai::build_context(&self.state),
        };

        let history_limit = self.state.config.ai.history_limit;
        self.state.ai.status = AiStatus::Queued;
        self.state.ai.push_request(request.clone(), history_limit);
        self.state.ai.push_message(
            AiMessage {
                role: AiMessageRole::User,
                content: prompt.clone(),
                created_at: now_utc(),
            },
            history_limit,
        );
        self.state.ai.active_request = Some(request.id);
        self.state.ai.last_error = None;
        self.append_timeline(
            TimelineKind::Ai,
            format!(
                "queued AI {} request #{}: {}",
                kind.label().to_ascii_lowercase(),
                request.id,
                truncate_for_timeline(&prompt, 96)
            ),
        );
        self.push_notification(
            NotificationLevel::Info,
            format!("queued AI {} request", kind.label().to_ascii_lowercase()),
        );
        vec![Effect::RunAiRequest(Box::new(request))]
    }

    fn push_shell_history(&mut self, entry: CommandHistoryEntry) {
        self.state.commands.history.push(entry);
        if self.state.commands.history.len() > self.state.config.commands.history_limit {
            let overflow =
                self.state.commands.history.len() - self.state.config.commands.history_limit;
            self.state.commands.history.drain(0..overflow);
        }
    }

    fn prepare_execution(
        &mut self,
        raw: String,
        background: bool,
        provenance: crate::domain::CommandProvenance,
    ) -> Vec<Effect> {
        self.prepare_execution_internal(raw, background, provenance, None, None, None)
    }

    fn prepare_execution_internal(
        &mut self,
        raw: String,
        background: bool,
        provenance: crate::domain::CommandProvenance,
        proposal_context: Option<(usize, &AiActionProposal)>,
        cwd_override: Option<PathBuf>,
        shell_override: Option<String>,
    ) -> Vec<Effect> {
        let safety_class = classify_command(&raw);
        let command_id = self.ids.next_command();
        let job_id = self.ids.next_job();
        let service_id = background.then(|| self.ids.next_service());
        let submitted_at = now_utc();
        let request = ExecutionRequest {
            command_id,
            job_id,
            service_id,
            raw: raw.clone(),
            cwd: cwd_override.unwrap_or_else(|| self.state.commands.session.cwd.clone()),
            env: self.state.commands.session.env.clone(),
            shell: shell_override.unwrap_or_else(|| self.state.commands.session.shell.clone()),
            mode: ExecutionMode::Managed,
            provenance,
            background,
            safety_class,
        };
        let approval_mode = approval_mode_for(safety_class, &self.state.config.safety);
        let status = match approval_mode {
            Some(ApprovalMode::InlineReview) => CommandStatus::PendingReview,
            Some(ApprovalMode::ModalConfirm) => CommandStatus::PendingApproval,
            None => CommandStatus::Queued,
        };

        self.push_shell_history(CommandHistoryEntry::from_request(
            &request,
            status,
            submitted_at,
        ));

        self.state
            .commands
            .records
            .push(CommandRecord::from_request(&request, status));
        self.state.jobs.records.push(JobRecord {
            id: job_id,
            command_id,
            label: raw.clone(),
            pid: None,
            background,
            status: JobStatus::Queued,
            started_at: None,
            ended_at: None,
        });

        if let Some(mode) = approval_mode {
            let approval = ApprovalRequest {
                id: self.ids.next_approval(),
                command_id,
                created_at: now_utc(),
                class: safety_class,
                mode,
                summary: raw.clone(),
                detail: approval_detail(background, safety_class, proposal_context),
                execution: request,
            };
            self.append_timeline(TimelineKind::Approval, approval_message(&approval));
            if matches!(approval.mode, ApprovalMode::ModalConfirm) {
                self.state.ui.modal = Some(ModalState::Approval(approval.id));
                self.state.ui.focus = FocusTarget::Modal;
            }
            self.state.approvals.pending.push(approval.clone());
            return vec![Effect::QueueApproval(approval)];
        }

        self.append_timeline(
            TimelineKind::Command,
            format!(
                "queued command #{command_id}: {}",
                truncate_for_timeline(&raw, 96)
            ),
        );
        vec![Effect::ExecuteCommand(request)]
    }

    fn apply_ai_proposal(&mut self, proposal_index: usize) -> Vec<Effect> {
        let Some(response) = self.state.ai.last_response.clone() else {
            self.push_notification(
                NotificationLevel::Warn,
                "no AI proposals are available yet".to_string(),
            );
            self.append_timeline(
                TimelineKind::Ai,
                "attempted to apply an AI proposal before any response was available".to_string(),
            );
            return Vec::new();
        };

        let Some(proposal) = response
            .proposals
            .get(proposal_index.saturating_sub(1))
            .cloned()
        else {
            self.push_notification(
                NotificationLevel::Warn,
                format!("AI proposal #{proposal_index} does not exist"),
            );
            self.append_timeline(
                TimelineKind::Ai,
                format!(
                    "attempted to apply missing AI proposal #{proposal_index} from request #{}",
                    response.request_id
                ),
            );
            return Vec::new();
        };

        let Some(command) = proposal
            .command
            .as_deref()
            .map(str::trim)
            .filter(|command| !command.is_empty())
        else {
            self.push_notification(
                NotificationLevel::Warn,
                format!("AI proposal #{proposal_index} has no executable command"),
            );
            self.append_timeline(
                TimelineKind::Ai,
                format!(
                    "AI proposal #{proposal_index} is advisory only: {}",
                    truncate_for_timeline(&proposal.summary, 96)
                ),
            );
            return Vec::new();
        };

        let parsed =
            match parse_shell_input(command, crate::domain::CommandProvenance::AiSuggestion) {
                Ok(parsed) => parsed,
                Err(message) => {
                    self.push_notification(NotificationLevel::Warn, message.clone());
                    self.append_timeline(TimelineKind::Error, message);
                    return Vec::new();
                }
            };

        self.append_timeline(
            TimelineKind::Ai,
            format!(
                "selected AI proposal #{proposal_index}: {}",
                truncate_for_timeline(&proposal.summary, 96)
            ),
        );
        self.push_notification(
            NotificationLevel::Info,
            format!("selected AI proposal #{proposal_index}"),
        );

        match parsed {
            ParsedInput::Execute {
                command,
                background,
                provenance,
            } => self.prepare_execution_internal(
                command,
                background,
                provenance,
                Some((proposal_index, &proposal)),
                None,
                None,
            ),
            _ => Vec::new(),
        }
    }

    fn resolve_approval(&mut self, approval_id: ApprovalId, approved: bool) -> Vec<Effect> {
        let position = self
            .state
            .approvals
            .pending
            .iter()
            .position(|request| request.id == approval_id);
        let Some(position) = position else {
            return Vec::new();
        };

        let request = self.state.approvals.pending.remove(position);
        self.state.approvals.history.push(ApprovalDecision {
            approval_id,
            command_id: request.command_id,
            approved,
            decided_at: now_utc(),
            note: None,
        });
        self.state.ui.modal = None;
        self.state.ui.focus = FocusTarget::CommandPane;

        if approved {
            if let Some(command) = self.find_command_mut(request.command_id) {
                command.status = CommandStatus::Queued;
            }
            self.update_history_for_command(request.command_id, |entry| {
                entry.status = CommandStatus::Queued;
            });
            self.append_timeline(
                TimelineKind::Approval,
                format!("approved command #{}", request.command_id),
            );
            return vec![Effect::ExecuteCommand(request.execution)];
        }

        let denied_at = now_utc();
        if let Some(command) = self.find_command_mut(request.command_id) {
            command.status = CommandStatus::Denied;
            command.ended_at = Some(denied_at);
        }
        self.update_history_for_command(request.command_id, |entry| {
            entry.status = CommandStatus::Denied;
            entry.ended_at = Some(denied_at);
        });
        if let Some(job) = self.find_job_by_command_mut(request.command_id) {
            job.status = JobStatus::Cancelled;
            job.ended_at = Some(denied_at);
        }
        self.append_timeline(
            TimelineKind::Approval,
            format!("denied command #{}", request.command_id),
        );
        Vec::new()
    }

    fn current_approval_id(&self) -> Option<ApprovalId> {
        if let Some(modal) = self.state.ui.modal.as_ref() {
            return match modal {
                ModalState::Approval(id) => Some(*id),
                ModalState::Help | ModalState::History | ModalState::Error(_) => None,
            };
        }
        self.current_inline_approval_id()
    }

    fn close_modal(&mut self) {
        self.state.ui.modal = None;
        self.state.ui.focus = FocusTarget::CommandPane;
    }

    fn current_inline_approval_id(&self) -> Option<ApprovalId> {
        self.state
            .approvals
            .pending
            .iter()
            .find(|request| matches!(request.mode, ApprovalMode::InlineReview))
            .map(|request| request.id)
    }

    fn append_timeline(&mut self, kind: TimelineKind, message: String) {
        self.state.timeline.entries.push(TimelineEntry {
            id: self.ids.next_timeline(),
            at: now_utc(),
            kind,
            message,
        });
    }

    fn push_notification(&mut self, level: NotificationLevel, message: String) {
        self.state.notifications.items.push(Notification {
            id: self.ids.next_notification(),
            level,
            message,
            created_at: now_utc(),
        });
        if self.state.notifications.items.len() > 10 {
            let overflow = self.state.notifications.items.len() - 10;
            self.state.notifications.items.drain(0..overflow);
        }
    }

    fn advance_tab(&mut self, forward: bool) {
        let tabs = DashboardTab::all();
        let current_index = tabs
            .iter()
            .position(|tab| *tab == self.state.ui.dashboard_tab)
            .unwrap_or(0);
        let next_index = if forward {
            (current_index + 1) % tabs.len()
        } else if current_index == 0 {
            tabs.len() - 1
        } else {
            current_index - 1
        };
        self.state.ui.dashboard_tab = tabs[next_index];
    }

    fn find_command(&self, id: CommandId) -> Option<&CommandRecord> {
        self.state
            .commands
            .records
            .iter()
            .find(|record| record.id == id)
    }

    fn find_command_mut(&mut self, id: CommandId) -> Option<&mut CommandRecord> {
        self.state
            .commands
            .records
            .iter_mut()
            .find(|record| record.id == id)
    }

    fn update_history_for_command(
        &mut self,
        id: CommandId,
        update: impl FnOnce(&mut CommandHistoryEntry),
    ) {
        if let Some(entry) = self
            .state
            .commands
            .history
            .iter_mut()
            .rev()
            .find(|entry| entry.command_id == Some(id))
        {
            update(entry);
        }
    }

    fn find_job_mut(&mut self, id: crate::shared::ids::JobId) -> Option<&mut JobRecord> {
        self.state
            .jobs
            .records
            .iter_mut()
            .find(|record| record.id == id)
    }

    fn find_job_by_command_mut(&mut self, id: CommandId) -> Option<&mut JobRecord> {
        self.state
            .jobs
            .records
            .iter_mut()
            .find(|record| record.command_id == id)
    }

    fn upsert_service(&mut self, service: ServiceRecord) {
        match self
            .state
            .services
            .registry
            .iter_mut()
            .find(|record| record.id == service.id)
        {
            Some(existing) => *existing = service,
            None => self.state.services.registry.push(service),
        }
    }

    fn find_service_mut(
        &mut self,
        id: crate::shared::ids::ServiceId,
    ) -> Option<&mut ServiceRecord> {
        self.state
            .services
            .registry
            .iter_mut()
            .find(|record| record.id == id)
    }

    fn upsert_process(&mut self, process: ProcessSnapshot) {
        match self
            .state
            .processes
            .snapshots
            .iter_mut()
            .find(|record| record.command_id == process.command_id)
        {
            Some(existing) => *existing = process,
            None => self.state.processes.snapshots.push(process),
        }
    }

    fn mark_process(&mut self, command_id: CommandId, status: ProcessStatus) {
        if let Some(process) = self
            .state
            .processes
            .snapshots
            .iter_mut()
            .find(|record| record.command_id == Some(command_id))
        {
            process.status = status;
            process.observed_at = now_utc();
        }
    }
}

fn truncate_for_timeline(message: &str, max_len: usize) -> String {
    let trimmed = message.trim();
    if trimmed.len() <= max_len {
        return trimmed.to_string();
    }
    format!("{}...", &trimmed[..max_len.saturating_sub(3)])
}

fn cd_history_entry(target: Option<&str>) -> String {
    target
        .map(str::trim)
        .filter(|target| !target.is_empty())
        .map(|target| format!("cd {target}"))
        .unwrap_or_else(|| "cd".to_string())
}

fn next_id_after_restored_state(state: &AppState) -> u64 {
    state
        .commands
        .history
        .iter()
        .filter_map(|entry| entry.command_id.map(|id| id.0))
        .chain(state.commands.records.iter().map(|record| record.id.0))
        .chain(state.jobs.records.iter().map(|record| record.id.0))
        .chain(state.services.registry.iter().map(|record| record.id.0))
        .chain(state.approvals.pending.iter().map(|record| record.id.0))
        .chain(state.timeline.entries.iter().map(|record| record.id.0))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn resolve_cd_target(
    current: &Path,
    previous: Option<&Path>,
    target: Option<&str>,
) -> Result<PathBuf, String> {
    let candidate = match target.map(str::trim).filter(|target| !target.is_empty()) {
        None => dirs::home_dir().ok_or_else(|| "cd: home directory is unavailable".to_string())?,
        Some("-") => previous
            .map(Path::to_path_buf)
            .ok_or_else(|| "cd: no previous directory recorded".to_string())?,
        Some(raw) => expand_cd_target(raw)?,
    };

    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        current.join(candidate)
    };

    let canonical = std::fs::canonicalize(&candidate)
        .map_err(|error| format!("cd: {}: {error}", candidate.display()))?;
    if !canonical.is_dir() {
        return Err(format!("cd: not a directory: {}", canonical.display()));
    }

    Ok(canonical)
}

fn expand_cd_target(raw: &str) -> Result<PathBuf, String> {
    if raw == "~" {
        return dirs::home_dir().ok_or_else(|| "cd: home directory is unavailable".to_string());
    }

    if let Some(rest) = raw.strip_prefix("~/").or_else(|| raw.strip_prefix("~\\")) {
        let home =
            dirs::home_dir().ok_or_else(|| "cd: home directory is unavailable".to_string())?;
        return Ok(home.join(rest));
    }

    Ok(PathBuf::from(raw))
}

fn approval_detail(
    background: bool,
    safety_class: crate::domain::SafetyClass,
    proposal_context: Option<(usize, &AiActionProposal)>,
) -> String {
    if let Some((proposal_index, proposal)) = proposal_context {
        return format!(
            "AI proposal #{proposal_index}: {}. {} Runtime classification: {} (model hint: {}).",
            proposal.summary,
            proposal.detail,
            safety_class.label(),
            proposal.safety_class.label()
        );
    }

    format!(
        "{} action classified as {}",
        if background {
            "background"
        } else {
            "foreground"
        },
        safety_class.label()
    )
}
