use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::ai::AiRuntime;
use crate::app::{AppAction, AppEvent, AppStore, Effect, UiIntent};
use crate::commands::{slash_palette_active, ParsedInput};
use crate::domain::FocusTarget;
use crate::observability::ObservabilityHandle;
use crate::runtime::RuntimeSupervisor;
use crate::shared::error::AppResult;
use crate::ui::{render, terminal::TerminalUi};

pub struct Runtime {
    store: AppStore,
    terminal: TerminalUi,
    supervisor: RuntimeSupervisor,
    events_rx: mpsc::Receiver<AppEvent>,
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
                                let effects = self.handle_terminal_event(event);
                                self.apply_effects(effects).await?;
                                self.render()?;
                            }
                            Err(error) => {
                                error!(%error, "terminal input error");
                                let effects = self.store.dispatch_event(AppEvent::Error(error.to_string()));
                                self.apply_effects(effects).await?;
                                self.render()?;
                            }
                        }
                    }
                }
                Some(event) = self.events_rx.recv() => {
                    let effects = self.store.dispatch_event(event);
                    self.apply_effects(effects).await?;
                    self.render()?;
                }
                _ = tick.tick() => {
                    let effects = self.store.dispatch_action(AppAction::Ui(UiIntent::Tick));
                    self.apply_effects(effects).await?;
                    self.render()?;
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
        self.terminal.restore()?;
        Ok(())
    }

    async fn apply_effects(&mut self, effects: Vec<Effect>) -> AppResult<()> {
        for effect in effects {
            match effect {
                Effect::RunAiRequest(request) => {
                    self.supervisor.run_ai_request(*request).await?;
                }
                Effect::ExecuteCommand(request) => {
                    self.supervisor.execute_command(request).await?;
                }
                Effect::CancelCommand(command_id) => {
                    self.supervisor.cancel_command(command_id).await?;
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

    fn render(&mut self) -> AppResult<()> {
        self.terminal
            .draw(|frame| render(frame, self.store.state()))?;
        Ok(())
    }

    fn handle_terminal_event(&mut self, event: Event) -> Vec<Effect> {
        let action = match event {
            Event::Key(key) => map_key_event(key, self.store.state()),
            Event::Resize(width, height) => Some(AppAction::Ui(UiIntent::Resize(width, height))),
            _ => None,
        };

        action
            .map(|action| self.store.dispatch_action(action))
            .unwrap_or_default()
    }
}

fn map_key_event(key: KeyEvent, state: &crate::domain::AppState) -> Option<AppAction> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }

    let command_palette_active = matches!(state.ui.focus, FocusTarget::CommandPane)
        && state.ui.modal.is_none()
        && slash_palette_active(&state.ui.input.buffer);

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
