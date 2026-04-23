use std::collections::VecDeque;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::ai::AiRuntime;
use crate::app::{AppAction, AppEvent, AppStore, Effect, UiIntent};
use crate::commands::{slash_palette_active, ParsedInput};
use crate::domain::{ExecutionMode, FocusTarget};
use crate::infra::{shell, state as local_state};
use crate::observability::ObservabilityHandle;
use crate::runtime::RuntimeSupervisor;
use crate::shared::error::{AppResult, InfraError, PtyPhase};
use crate::ui::{render, terminal::TerminalUi};

pub struct Runtime {
    store: AppStore,
    terminal: TerminalUi,
    supervisor: RuntimeSupervisor,
    events_rx: mpsc::Receiver<AppEvent>,
    active_pty: Option<shell::AttachedPtySession>,
    _observability: ObservabilityHandle,
}

impl Runtime {
    pub fn new(
        store: AppStore,
        terminal: TerminalUi,
        observability: ObservabilityHandle,
        ai_runtime: AiRuntime,
    ) -> Self {
        let (events_tx, events_rx) = mpsc::channel(512);
        let supervisor = RuntimeSupervisor::new(events_tx, ai_runtime);
        Self {
            store,
            terminal,
            supervisor,
            events_rx,
            active_pty: None,
            _observability: observability,
        }
    }

    pub async fn run(mut self) -> AppResult<()> {
        info!("forge runtime starting");
        self.supervisor
            .refresh_git(self.store.state().project.root.clone());
        self.render()?;

        let mut tick = tokio::time::interval(Duration::from_millis(
            self.store.state().config.ui.tick_rate_ms,
        ));
        let mut input_events = EventStream::new();

        loop {
            if self.store.state().app.should_quit {
                break;
            }

            tokio::select! {
                maybe_event = input_events.next() => {
                    if let Some(result) = maybe_event {
                        match result {
                            Ok(event) => {
                                let effects = self.handle_terminal_event(event)?;
                                self.apply_effects(effects).await?;
                                if !self.pty_active() {
                                    self.render()?;
                                }
                            }
                            Err(error) => {
                                error!(%error, "terminal input error");
                                let effects = self.store.dispatch_event(AppEvent::Error(error.to_string()));
                                self.apply_effects(effects).await?;
                                if !self.pty_active() {
                                    self.render()?;
                                }
                            }
                        }
                    }
                }
                Some(event) = self.events_rx.recv() => {
                    let effects = self.process_app_event(event)?;
                    self.apply_effects(effects).await?;
                    if !self.pty_active() {
                        self.render()?;
                    }
                }
                _ = tick.tick() => {
                    let effects = self.store.dispatch_action(AppAction::Ui(UiIntent::Tick));
                    self.apply_effects(effects).await?;
                    if !self.pty_active() {
                        self.render()?;
                    }
                }
                signal = tokio::signal::ctrl_c() => {
                    if signal.is_ok() {
                        let effects = self.store.dispatch_action(AppAction::Ui(UiIntent::Quit));
                        self.apply_effects(effects).await?;
                    }
                }
            }
        }

        info!("forge runtime shutting down");
        self.shutdown_active_pty().await?;
        let persist_result = self.persist_local_state();
        self.terminal.restore()?;
        persist_result?;
        Ok(())
    }

    fn persist_local_state(&self) -> AppResult<()> {
        if let Some(path) = local_state::save_command_history(
            &self.store.state().project.root,
            &self.store.state().commands.history,
        )? {
            info!(path = %path.display(), "command history persisted");
        }
        Ok(())
    }

    async fn apply_effects(&mut self, effects: Vec<Effect>) -> AppResult<()> {
        let mut pending = VecDeque::from(effects);
        while let Some(effect) = pending.pop_front() {
            match effect {
                Effect::RunAiRequest(request) => {
                    self.supervisor.run_ai_request(*request).await?;
                }
                Effect::ExecuteCommand(request) => {
                    if matches!(request.mode, ExecutionMode::Pty) {
                        self.start_pty_command(request)?;
                    } else {
                        self.supervisor.execute_command(request).await?;
                    }
                }
                Effect::CancelCommand(command_id) => {
                    if self
                        .active_pty
                        .as_ref()
                        .is_some_and(|session| session.command_id() == command_id)
                    {
                        self.active_pty
                            .as_ref()
                            .expect("active pty should exist")
                            .cancel()?;
                    } else {
                        self.supervisor.cancel_command(command_id).await?;
                    }
                }
                Effect::RefreshProjectContext(path) => {
                    self.supervisor.refresh_project(path);
                }
                Effect::RefreshGit(path) => {
                    self.supervisor.refresh_git(path);
                }
                Effect::Shutdown => {
                    self.store.state_mut().app.should_quit = true;
                }
                Effect::QueueApproval(_request) => {}
            }
        }
        Ok(())
    }

    fn start_pty_command(&mut self, request: crate::domain::ExecutionRequest) -> AppResult<()> {
        self.terminal.begin_pty_session()?;
        match shell::spawn_attached_pty_command(request, self.supervisor.events_tx()) {
            Ok(session) => {
                self.active_pty = Some(session);
                Ok(())
            }
            Err(error) => {
                self.terminal.end_pty_session()?;
                Err(error)
            }
        }
    }

    fn render(&mut self) -> AppResult<()> {
        self.terminal
            .draw(|frame| render(frame, self.store.state()))?;
        Ok(())
    }

    fn process_app_event(&mut self, event: AppEvent) -> AppResult<Vec<Effect>> {
        let should_finish = self
            .active_pty
            .as_ref()
            .is_some_and(|session| matches!(
                event,
                AppEvent::CommandExited { command_id, .. } | AppEvent::CommandCancelled { command_id, .. }
                    if command_id == session.command_id()
            ));

        let effects = self.store.dispatch_event(event);
        if should_finish {
            self.finish_pty_session()?;
        }
        Ok(effects)
    }

    fn handle_terminal_event(&mut self, event: Event) -> AppResult<Vec<Effect>> {
        if self.pty_active() {
            self.handle_pty_terminal_event(event)?;
            return Ok(Vec::new());
        }

        let action = match event {
            Event::Key(key) => map_key_event(key, self.store.state()),
            Event::Resize(width, height) => Some(AppAction::Ui(UiIntent::Resize(width, height))),
            _ => None,
        };

        Ok(action
            .map(|action| self.store.dispatch_action(action))
            .unwrap_or_default())
    }

    fn handle_pty_terminal_event(&mut self, event: Event) -> AppResult<()> {
        let Some(session) = self.active_pty.as_ref() else {
            return Ok(());
        };

        match event {
            Event::Key(key) => {
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    return Ok(());
                }

                if matches!(key.code, KeyCode::Char('c'))
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    session.interrupt()?;
                    return Ok(());
                }

                if let Some(bytes) = encode_pty_key(key) {
                    session.send_input(bytes)?;
                }
            }
            Event::Resize(width, height) => {
                session.resize(width, height)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn finish_pty_session(&mut self) -> AppResult<()> {
        if self.active_pty.take().is_some() {
            self.terminal.end_pty_session()?;
        }
        Ok(())
    }

    async fn shutdown_active_pty(&mut self) -> AppResult<()> {
        if let Some(session) = self.active_pty.as_ref() {
            session.cancel()?;
        } else {
            return Ok(());
        }

        let timeout = tokio::time::sleep(Duration::from_secs(2));
        tokio::pin!(timeout);
        loop {
            if !self.pty_active() {
                return Ok(());
            }

            tokio::select! {
                _ = &mut timeout => {
                    return Err(InfraError::Pty {
                        phase: PtyPhase::Shutdown,
                        message: "timed out waiting for PTY session shutdown".to_string(),
                    }
                    .into());
                }
                maybe_event = self.events_rx.recv() => {
                    if let Some(event) = maybe_event {
                        let effects = self.process_app_event(event)?;
                        self.apply_effects(effects).await?;
                    }
                }
            }
        }
    }

    fn pty_active(&self) -> bool {
        self.active_pty.is_some()
    }
}

fn encode_pty_key(key: KeyEvent) -> Option<Vec<u8>> {
    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return control_char_bytes(c);
            }

            let mut bytes = Vec::new();
            if key.modifiers.contains(KeyModifiers::ALT) {
                bytes.push(0x1b);
            }
            let mut encoded = [0_u8; 4];
            let slice = c.encode_utf8(&mut encoded);
            bytes.extend_from_slice(slice.as_bytes());
            Some(bytes)
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::Backspace => Some(vec![0x7f]),
        KeyCode::Esc => Some(vec![0x1b]),
        KeyCode::Left => Some(b"\x1b[D".to_vec()),
        KeyCode::Right => Some(b"\x1b[C".to_vec()),
        KeyCode::Up => Some(b"\x1b[A".to_vec()),
        KeyCode::Down => Some(b"\x1b[B".to_vec()),
        KeyCode::Home => Some(b"\x1b[H".to_vec()),
        KeyCode::End => Some(b"\x1b[F".to_vec()),
        KeyCode::Delete => Some(b"\x1b[3~".to_vec()),
        KeyCode::PageUp => Some(b"\x1b[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1b[6~".to_vec()),
        _ => None,
    }
}

fn control_char_bytes(c: char) -> Option<Vec<u8>> {
    if !c.is_ascii() {
        return None;
    }

    let upper = c.to_ascii_uppercase();
    if upper == ' ' {
        return Some(vec![0x00]);
    }
    if ('@'..='_').contains(&upper) {
        return Some(vec![(upper as u8) - b'@']);
    }
    None
}

fn map_key_event(key: KeyEvent, state: &crate::domain::AppState) -> Option<AppAction> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }

    let command_palette_active = matches!(state.ui.focus, FocusTarget::CommandPane)
        && state.ui.modal.is_none()
        && (slash_palette_active(&state.ui.input.buffer)
            || !crate::ui::views::intent_suggestions(state).is_empty());

    if matches!(key.code, KeyCode::F(1)) {
        return Some(AppAction::ParsedInput(ParsedInput::Help));
    }

    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Ui(UiIntent::Quit))
        }
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Ui(UiIntent::ClearInput))
        }
        KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Ui(UiIntent::InsertNewline))
        }
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Ui(UiIntent::RecallPreviousHistory))
        }
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(AppAction::Ui(UiIntent::RecallNextHistory))
        }
        KeyCode::Tab if command_palette_active => {
            Some(AppAction::Ui(UiIntent::AcceptCommandSuggestion))
        }
        KeyCode::Tab => Some(AppAction::Ui(UiIntent::NextFocus)),
        KeyCode::BackTab => Some(AppAction::Ui(UiIntent::PrevFocus)),
        KeyCode::Up if command_palette_active => {
            Some(AppAction::Ui(UiIntent::PrevCommandSuggestion))
        }
        KeyCode::Up
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::MoveCursorUp))
        }
        KeyCode::Down if command_palette_active => {
            Some(AppAction::Ui(UiIntent::NextCommandSuggestion))
        }
        KeyCode::Down
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::MoveCursorDown))
        }
        KeyCode::Left
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::MoveCursorLeft))
        }
        KeyCode::Right
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::MoveCursorRight))
        }
        KeyCode::Home
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::MoveCursorHome))
        }
        KeyCode::End
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::MoveCursorEnd))
        }
        KeyCode::Left
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::DashboardPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::PrevTab))
        }
        KeyCode::Right
            if key.modifiers.is_empty()
                && matches!(state.ui.focus, FocusTarget::DashboardPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::NextTab))
        }
        KeyCode::Left if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(AppAction::Ui(UiIntent::PrevTab))
        }
        KeyCode::Right if key.modifiers.contains(KeyModifiers::ALT) => {
            Some(AppAction::Ui(UiIntent::NextTab))
        }
        KeyCode::F(2)
            if matches!(state.ui.focus, FocusTarget::CommandPane) && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::CycleInputMode))
        }
        KeyCode::Enter
            if key.modifiers.contains(KeyModifiers::SHIFT)
                && matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::Ui(UiIntent::InsertNewline))
        }
        KeyCode::Enter => Some(AppAction::Ui(UiIntent::Submit)),
        KeyCode::Backspace => Some(AppAction::Ui(UiIntent::Backspace)),
        KeyCode::Esc => Some(AppAction::Ui(UiIntent::Esc)),
        KeyCode::Char('?')
            if key.modifiers == KeyModifiers::SHIFT
                && !matches!(state.ui.focus, FocusTarget::CommandPane)
                && state.ui.modal.is_none() =>
        {
            Some(AppAction::ParsedInput(ParsedInput::Help))
        }
        KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            Some(AppAction::Ui(UiIntent::KeyChar(c)))
        }
        _ => None,
    }
}
