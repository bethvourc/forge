use std::collections::BTreeMap;
use std::time::SystemTime;

use ratatui::{backend::TestBackend, style::Modifier, Terminal};

use forge::app::{AppAction, AppEvent, AppStore, Effect};
use forge::commands::{parse_input, slash_command_suggestions, ParsedInput};
use forge::config::ForgeConfig;
use forge::domain::{
    AiActionProposal, AiRequestKind, AiResponse, AppState, ApprovalMode, CommandHistoryEntry,
    CommandProvenance, CommandStatus, DashboardTab, DiagnosticLevel, ExecutionMode, FocusTarget,
    GitSnapshot, InputMode, LogEntry, LogSeverity, LogSource, LogStream, ModalState,
    ProjectContext, SafetyClass, ShellSessionState,
};
use forge::safety::classify_command;
use forge::shared::ids::{AiRequestId, CommandId, LogId};
use forge::shared::time::now_utc;

#[test]
fn parses_background_shell_execution() {
    let parsed = parse_input("cargo test &").expect("background shell command should parse");
    match parsed {
        ParsedInput::Execute {
            command,
            background,
            provenance,
        } => {
            assert_eq!(command, "cargo test");
            assert!(background);
            assert!(matches!(provenance, CommandProvenance::UserInput));
        }
        other => panic!("unexpected parse result: {:?}", other),
    }
}

#[test]
fn parses_plain_clear_as_forge_builtin() {
    assert!(matches!(parse_input("clear").unwrap(), ParsedInput::Clear));
}

#[test]
fn parses_cd_as_forge_builtin() {
    match parse_input("cd src").expect("cd should parse as a builtin") {
        ParsedInput::ChangeDirectory { target } => {
            assert_eq!(target.as_deref(), Some("src"));
        }
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("cd \"path with spaces\"").expect("quoted cd target should parse") {
        ParsedInput::ChangeDirectory { target } => {
            assert_eq!(target.as_deref(), Some("path with spaces"));
        }
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("cd src && pwd").expect("compound shell command should remain shell input") {
        ParsedInput::Execute { command, .. } => assert_eq!(command, "cd src && pwd"),
        other => panic!("unexpected parse result: {:?}", other),
    }
}

#[test]
fn parses_session_env_builtins() {
    match parse_input("export FORGE_MODE=local").expect("export should parse as a session builtin")
    {
        ParsedInput::SetEnv { key, value } => {
            assert_eq!(key, "FORGE_MODE");
            assert_eq!(value, "local");
        }
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("unset FORGE_MODE").expect("unset should parse as a session builtin") {
        ParsedInput::UnsetEnv { key } => assert_eq!(key, "FORGE_MODE"),
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("export 1BAD=value").expect("invalid export should remain shell input") {
        ParsedInput::Execute { command, .. } => assert_eq!(command, "export 1BAD=value"),
        other => panic!("unexpected parse result: {:?}", other),
    }
}

#[test]
fn parses_supported_slash_commands() {
    assert!(matches!(parse_input("/quit").unwrap(), ParsedInput::Quit));
    assert!(matches!(parse_input("/exit").unwrap(), ParsedInput::Quit));
    assert!(matches!(parse_input("exit").unwrap(), ParsedInput::Quit));
    assert!(matches!(parse_input("quit").unwrap(), ParsedInput::Quit));
    assert!(matches!(parse_input("/clear").unwrap(), ParsedInput::Clear));
    assert!(matches!(
        parse_input("/tab next").unwrap(),
        ParsedInput::NextTab
    ));
    assert!(matches!(
        parse_input("/next-tab").unwrap(),
        ParsedInput::NextTab
    ));
    assert!(matches!(
        parse_input("/tab prev").unwrap(),
        ParsedInput::PrevTab
    ));
    assert!(matches!(
        parse_input("/prev-tab").unwrap(),
        ParsedInput::PrevTab
    ));
    assert!(matches!(
        parse_input("/approve").unwrap(),
        ParsedInput::ApprovePending
    ));
    assert!(matches!(
        parse_input("/deny").unwrap(),
        ParsedInput::DenyPending
    ));
    assert!(matches!(
        parse_input("/history").unwrap(),
        ParsedInput::History
    ));
    assert!(matches!(
        parse_input("/rerun-last").unwrap(),
        ParsedInput::RerunLast
    ));
    assert!(matches!(
        parse_input("/rerun 12").unwrap(),
        ParsedInput::Rerun(CommandId(12))
    ));

    match parse_input("/bg cargo run").unwrap() {
        ParsedInput::Execute {
            command,
            background,
            provenance,
        } => {
            assert_eq!(command, "cargo run");
            assert!(background);
            assert!(matches!(provenance, CommandProvenance::SlashCommand));
        }
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("/ai summarize logs").unwrap() {
        ParsedInput::AiPrompt { prompt, kind } => {
            assert_eq!(prompt, "summarize logs");
            assert!(matches!(kind, AiRequestKind::Assist));
        }
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("/diagnose failing test").unwrap() {
        ParsedInput::AiPrompt { prompt, kind } => {
            assert_eq!(prompt, "failing test");
            assert!(matches!(kind, AiRequestKind::Diagnose));
        }
        other => panic!("unexpected parse result: {:?}", other),
    }

    match parse_input("/apply 2").unwrap() {
        ParsedInput::ApplyAiProposal(index) => assert_eq!(index, 2),
        other => panic!("unexpected parse result: {:?}", other),
    }

    assert!(parse_input("/apply 0").is_err());

    let suggestions = slash_command_suggestions("/");
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|command| command.usage == "/help"));
}

#[test]
fn parses_cancel_slash_command() {
    let parsed = parse_input("/cancel 42").expect("cancel slash command should parse");
    match parsed {
        ParsedInput::Cancel(id) => assert_eq!(id.0, 42),
        other => panic!("unexpected parse result: {:?}", other),
    }
}

#[test]
fn parses_help_slash_command() {
    let parsed = parse_input("/help").expect("help slash command should parse");
    match parsed {
        ParsedInput::Help => {}
        other => panic!("unexpected parse result: {:?}", other),
    }
}

#[test]
fn safety_classifier_marks_destructive_commands() {
    let class = classify_command("rm -rf build");
    assert!(matches!(class, SafetyClass::Destructive));
}

#[test]
fn store_help_command_opens_modal() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Help));

    assert!(effects.is_empty());
    assert!(matches!(store.state().ui.modal, Some(ModalState::Help)));
    assert!(matches!(store.state().ui.focus, FocusTarget::Modal));
}

#[test]
fn store_executes_passive_shell_command_immediately() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "pwd".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    assert_eq!(effects.len(), 1);
    let command = store
        .state()
        .commands
        .records
        .last()
        .expect("command record should exist");
    assert_eq!(command.raw, "pwd");
    assert!(matches!(command.status, CommandStatus::Queued));
    assert!(matches!(
        effects.as_slice(),
        [Effect::ExecuteCommand(request)] if matches!(request.mode, ExecutionMode::Managed)
    ));
}

#[test]
fn store_routes_interactive_foreground_commands_to_pty() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "top".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    assert!(matches!(
        effects.as_slice(),
        [Effect::ExecuteCommand(request)] if matches!(request.mode, ExecutionMode::Pty)
    ));
}

#[test]
fn store_routes_caution_commands_to_inline_review() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "cargo test".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    assert_eq!(effects.len(), 1);
    let approval = store
        .state()
        .approvals
        .pending
        .last()
        .expect("approval should be queued");
    assert!(matches!(approval.mode, ApprovalMode::InlineReview));
    let command = store
        .state()
        .commands
        .records
        .last()
        .expect("command record should exist");
    assert!(matches!(command.status, CommandStatus::PendingReview));
}

#[test]
fn store_routes_risky_commands_to_modal_approval() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "echo hi > /tmp/forge-test.txt".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    assert_eq!(effects.len(), 1);
    let approval = store
        .state()
        .approvals
        .pending
        .last()
        .expect("approval should be queued");
    assert!(matches!(approval.mode, ApprovalMode::ModalConfirm));
    assert!(matches!(
        store.state().ui.modal,
        Some(ModalState::Approval(_))
    ));
}

#[test]
fn store_ai_prompt_reports_unavailable_provider() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::AiPrompt {
        prompt: "explain the failure".to_string(),
        kind: AiRequestKind::Diagnose,
    }));

    assert!(matches!(effects.as_slice(), [Effect::RunAiRequest(_)]));
    assert_eq!(store.state().ai.requests.len(), 1);
    assert!(store.state().ai.active_request.is_some());
    assert!(matches!(
        store.state().ai.requests[0].kind,
        AiRequestKind::Diagnose
    ));
}

#[test]
fn store_apply_ai_proposal_uses_runtime_safety_pipeline() {
    let mut store = test_store();
    store.state_mut().ai.last_response = Some(AiResponse {
        request_id: AiRequestId(7),
        created_at: now_utc(),
        provider: "mock".to_string(),
        model: Some("test-model".to_string()),
        summary: "Suggested a follow-up action".to_string(),
        message: "The AI found a command worth reviewing.".to_string(),
        recommendations: Vec::new(),
        proposals: vec![AiActionProposal {
            summary: "Write a temp file".to_string(),
            detail: "Capture output in a temp file for later inspection.".to_string(),
            command: Some("echo hi > /tmp/forge-test.txt".to_string()),
            safety_class: SafetyClass::Passive,
        }],
        citations: Vec::new(),
    });

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::ApplyAiProposal(1)));

    assert!(matches!(effects.as_slice(), [Effect::QueueApproval(_)]));
    let command = store
        .state()
        .commands
        .records
        .last()
        .expect("proposal command record should exist");
    assert_eq!(command.raw, "echo hi > /tmp/forge-test.txt");
    assert!(matches!(
        command.provenance,
        CommandProvenance::AiSuggestion
    ));
    assert!(matches!(command.safety_class, SafetyClass::Risky));
    assert!(matches!(command.status, CommandStatus::PendingApproval));

    let approval = store
        .state()
        .approvals
        .pending
        .last()
        .expect("proposal approval should be queued");
    assert!(matches!(approval.mode, ApprovalMode::ModalConfirm));
    assert!(approval.detail.contains("AI proposal #1"));
    assert!(approval.detail.contains("model hint: Passive"));
}

#[test]
fn store_accepts_slash_command_suggestion_into_input() {
    let mut store = test_store();

    store.dispatch_action(AppAction::Ui(forge::app::UiIntent::KeyChar('/')));
    store.dispatch_action(AppAction::Ui(forge::app::UiIntent::KeyChar('d')));
    store.dispatch_action(AppAction::Ui(forge::app::UiIntent::AcceptCommandSuggestion));

    assert_eq!(store.state().ui.input.buffer, "/diagnose ");
    assert_eq!(store.state().ui.command_palette_cursor, 0);
}

#[test]
fn store_clear_resets_input_and_visible_logs() {
    let mut store = test_store();
    store.state_mut().ui.input.set_buffer("pwd".to_string());
    store.state_mut().logs.recent.push(LogEntry {
        id: LogId(1),
        ts: SystemTime::now(),
        source: LogSource::Command(CommandId(1)),
        stream: LogStream::Stdout,
        service_id: None,
        command_id: Some(CommandId(1)),
        severity: LogSeverity::Info,
        raw: "hello".to_string(),
        fields: BTreeMap::new(),
    });

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Clear));

    assert!(effects.is_empty());
    assert!(store.state().ui.input.buffer.is_empty());
    assert!(store.state().logs.recent.is_empty());
}

#[test]
fn store_cd_changes_session_cwd_without_command_record() {
    let mut store = test_store();
    let expected = std::env::current_dir()
        .expect("current dir should exist")
        .join("src")
        .canonicalize()
        .expect("src dir should exist");

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::ChangeDirectory {
        target: Some("src".to_string()),
    }));

    assert!(effects.is_empty());
    assert_eq!(store.state().commands.session.cwd, expected);
    assert_eq!(store.state().commands.session.last_exit_status, Some(0));
    let history = store.state().commands.history.last().unwrap();
    assert_eq!(history.raw, "cd src");
    assert_eq!(history.status, CommandStatus::Succeeded);
    assert_eq!(history.exit_code, Some(0));
    assert!(store.state().commands.records.is_empty());
    assert!(store.state().jobs.records.is_empty());
}

#[test]
fn store_failed_cd_surfaces_diagnostic_without_changing_cwd() {
    let mut store = test_store();
    let before = store.state().commands.session.cwd.clone();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::ChangeDirectory {
        target: Some("does-not-exist-for-forge-session".to_string()),
    }));

    assert!(effects.is_empty());
    assert_eq!(store.state().commands.session.cwd, before);
    assert_eq!(store.state().commands.session.last_exit_status, Some(1));
    assert!(store.state().commands.records.is_empty());
    let diagnostic = store
        .state()
        .diagnostics
        .records
        .iter()
        .last()
        .expect("failed cd should record a diagnostic");
    assert!(matches!(diagnostic.level, DiagnosticLevel::Warn));
    assert!(diagnostic.message.contains("cd:"));
}

#[test]
fn store_executes_commands_from_session_cwd() {
    let mut store = test_store();
    store.dispatch_action(AppAction::ParsedInput(ParsedInput::ChangeDirectory {
        target: Some("src".to_string()),
    }));
    let expected_cwd = store.state().commands.session.cwd.clone();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "pwd".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    match effects.as_slice() {
        [Effect::ExecuteCommand(request)] => {
            assert_eq!(request.cwd, expected_cwd);
            assert_eq!(request.shell, store.state().commands.session.shell);
        }
        other => panic!("unexpected effects: {:?}", other),
    }
}

#[test]
fn store_executes_commands_with_session_env_overlay() {
    let mut store = test_store();
    let set_effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::SetEnv {
        key: "FORGE_MODE".to_string(),
        value: "local".to_string(),
    }));
    assert!(set_effects.is_empty());

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "echo $FORGE_MODE".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    match effects.as_slice() {
        [Effect::ExecuteCommand(request)] => {
            assert_eq!(
                request.env.get("FORGE_MODE").map(String::as_str),
                Some("local")
            );
        }
        other => panic!("unexpected effects: {:?}", other),
    }

    let unset_effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::UnsetEnv {
        key: "FORGE_MODE".to_string(),
    }));
    assert!(unset_effects.is_empty());
    assert!(!store
        .state()
        .commands
        .session
        .env
        .contains_key("FORGE_MODE"));
    assert_eq!(store.state().commands.session.last_exit_status, Some(0));
}

#[test]
fn store_history_command_opens_modal() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::History));

    assert!(effects.is_empty());
    assert!(matches!(store.state().ui.modal, Some(ModalState::History)));
    assert!(matches!(store.state().ui.focus, FocusTarget::Modal));
}

#[test]
fn command_history_preserves_execution_metadata() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "pwd".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    assert!(matches!(effects.as_slice(), [Effect::ExecuteCommand(_)]));
    let command = store.state().commands.records.last().unwrap();
    let history = store.state().commands.history.last().unwrap();
    assert_eq!(history.command_id, Some(command.id));
    assert_eq!(history.raw, "pwd");
    assert_eq!(history.cwd, command.cwd);
    assert_eq!(history.shell, store.state().commands.session.shell);
    assert_eq!(history.provenance, CommandProvenance::UserInput);
    assert_eq!(history.safety_class, SafetyClass::Passive);
    assert_eq!(history.status, CommandStatus::Queued);
}

#[test]
fn command_exit_updates_session_last_status() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "pwd".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    let [Effect::ExecuteCommand(request)] = effects.as_slice() else {
        panic!("expected execute command effect: {:?}", effects);
    };

    store.dispatch_event(AppEvent::CommandExited {
        command_id: request.command_id,
        job_id: request.job_id,
        service_id: request.service_id,
        exit_code: 7,
    });

    assert_eq!(store.state().commands.session.last_exit_status, Some(7));
    let history = store.state().commands.history.last().unwrap();
    assert_eq!(history.status, CommandStatus::Failed);
    assert_eq!(history.exit_code, Some(7));
}

#[test]
fn store_rerun_last_uses_recorded_cwd_and_safety_pipeline() {
    let mut store = test_store();
    let replay_cwd = std::env::current_dir()
        .expect("current dir should exist")
        .join("src")
        .canonicalize()
        .expect("src dir should exist");
    store.state_mut().commands.history = vec![history_entry_with(
        Some(CommandId(42)),
        "pwd",
        replay_cwd.clone(),
        false,
        SafetyClass::Passive,
    )];

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::RerunLast));

    match effects.as_slice() {
        [Effect::ExecuteCommand(request)] => {
            assert_eq!(request.raw, "pwd");
            assert_eq!(request.cwd, replay_cwd);
            assert!(matches!(
                request.provenance,
                CommandProvenance::SlashCommand
            ));
        }
        other => panic!("unexpected effects: {:?}", other),
    }
}

#[test]
fn store_rerun_risky_command_requires_approval_again() {
    let mut store = test_store();
    let replay_cwd = store.state().commands.session.cwd.clone();
    store.state_mut().commands.history = vec![history_entry_with(
        Some(CommandId(42)),
        "echo hi > /tmp/forge-test.txt",
        replay_cwd.clone(),
        false,
        SafetyClass::Risky,
    )];

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Rerun(CommandId(42))));

    assert!(matches!(effects.as_slice(), [Effect::QueueApproval(_)]));
    let approval = store.state().approvals.pending.last().unwrap();
    assert_eq!(approval.execution.raw, "echo hi > /tmp/forge-test.txt");
    assert_eq!(approval.execution.cwd, replay_cwd);
    assert!(matches!(approval.mode, ApprovalMode::ModalConfirm));
}

#[test]
fn restored_history_advances_new_command_ids() {
    let mut state = AppState::new(
        ForgeConfig::default(),
        ProjectContext::default(),
        GitSnapshot::default(),
    );
    state.commands.history = vec![history_entry_with(
        Some(CommandId(42)),
        "pwd",
        state.commands.session.cwd.clone(),
        false,
        SafetyClass::Passive,
    )];
    let mut store = AppStore::new(state);

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Execute {
        command: "pwd".to_string(),
        background: false,
        provenance: CommandProvenance::UserInput,
    }));

    let [Effect::ExecuteCommand(request)] = effects.as_slice() else {
        panic!("expected execute command effect: {:?}", effects);
    };
    assert!(request.command_id.0 > 42);
}

#[test]
fn ai_context_includes_shell_session_state() {
    let mut store = test_store();
    store.dispatch_action(AppAction::ParsedInput(ParsedInput::ChangeDirectory {
        target: Some("src".to_string()),
    }));

    let context = forge::ai::build_context(store.state());

    assert_eq!(context.session.cwd, store.state().commands.session.cwd);
    assert_eq!(context.session.shell, store.state().commands.session.shell);
    assert_eq!(context.session.last_exit_status, Some(0));
}

#[test]
fn store_submits_plain_prompt_in_ai_mode() {
    let mut store = test_store();
    store.state_mut().ui.input.set_mode(InputMode::AiAssist);
    store
        .state_mut()
        .ui
        .input
        .set_buffer("summarize the latest failure".to_string());

    let effects = store.dispatch_action(AppAction::Ui(forge::app::UiIntent::Submit));

    assert!(matches!(effects.as_slice(), [Effect::RunAiRequest(_)]));
    assert_eq!(store.state().ai.requests.len(), 1);
    assert!(matches!(
        store.state().ai.requests[0].kind,
        AiRequestKind::Assist
    ));
}

#[test]
fn store_recalls_shell_history() {
    let mut store = test_store();
    store.state_mut().commands.history = vec![history_entry("pwd"), history_entry("cargo test")];
    store.dispatch_action(AppAction::Ui(forge::app::UiIntent::RecallPreviousHistory));
    assert_eq!(store.state().ui.input.buffer, "cargo test");

    store.dispatch_action(AppAction::Ui(forge::app::UiIntent::RecallNextHistory));
    assert!(store.state().ui.input.buffer.is_empty());
}

#[test]
fn store_quit_requests_shutdown() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Quit));

    assert!(store.state().app.should_quit);
    assert!(matches!(effects.as_slice(), [Effect::Shutdown]));
}

#[test]
fn store_tab_commands_cycle_dashboard_tabs() {
    let mut store = test_store();

    let next_effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::NextTab));
    assert!(next_effects.is_empty());
    assert!(matches!(
        store.state().ui.dashboard_tab,
        DashboardTab::Processes
    ));

    let prev_effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::PrevTab));
    assert!(prev_effects.is_empty());
    assert!(matches!(
        store.state().ui.dashboard_tab,
        DashboardTab::Services
    ));
}

#[test]
fn store_cancel_command_emits_effect() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::Cancel(CommandId(42))));

    assert!(matches!(
        effects.as_slice(),
        [Effect::CancelCommand(CommandId(42))]
    ));
}

#[test]
fn store_approve_without_pending_shows_notice() {
    let mut store = test_store();

    let effects = store.dispatch_action(AppAction::ParsedInput(ParsedInput::ApprovePending));

    assert!(effects.is_empty());
    assert!(store
        .state()
        .notifications
        .items
        .last()
        .is_some_and(|notice| notice.message.contains("no pending approvals")));
}

#[test]
fn ui_renders_prompt_first_wireframe_details() {
    let mut state = AppState::new(
        ForgeConfig::default(),
        ProjectContext::default(),
        GitSnapshot {
            branch: Some("forge/foundation".to_string()),
            head: Some("adc2ae2".to_string()),
            ..GitSnapshot::default()
        },
    );
    state.project.name = "forge".to_string();

    let mut terminal = Terminal::new(TestBackend::new(140, 44)).unwrap();
    terminal
        .draw(|frame| forge::ui::render(frame, &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rows = buffer_rows(buffer);
    assert!(
        rows[1].contains("forge   |   forge/foundation"),
        "status identity should show the app name and full branch"
    );
    assert!(
        rows[2].contains("────"),
        "status bar should have a visible underline separator"
    );

    let (question_x, question_y) = find_text(&rows, "what do you want to do?")
        .expect("empty command pane should render the wireframe question");
    let question_cell = buffer.cell((question_x, question_y)).unwrap();
    assert!(question_cell.modifier.contains(Modifier::ITALIC));
    assert!(question_cell.modifier.contains(Modifier::BOLD));

    assert!(
        rows.iter()
            .any(|row| row.contains("╭") && row.contains("╮")),
        "prompt box should use rounded wireframe corners"
    );
    assert!(
        rows.iter()
            .any(|row| row.contains("▸  run, ask, commit, deploy…█")),
        "prompt box should keep the uploaded placeholder and cursor treatment"
    );

    let (prompt_x, prompt_y) =
        find_symbol(buffer, "╭").expect("prompt box should have a top-left corner");
    let (prompt_right_x, _) = find_symbol_on_row(buffer, prompt_y, "╮")
        .expect("prompt box should have a top-right corner");
    assert_eq!(
        buffer.cell((prompt_right_x + 1, prompt_y + 1)).unwrap().bg,
        forge::ui::theme::BG_SHADOW,
        "prompt box should retain the subtle right-side shadow"
    );
    assert!(
        prompt_x > 0,
        "prompt box should be centered rather than pinned to the viewport edge"
    );
}

#[test]
fn command_history_persists_as_jsonl() {
    let root = unique_temp_root();
    let entries = vec![history_entry("pwd")];

    let path = forge::infra::state::save_command_history(&root, &entries)
        .expect("history should save")
        .expect("history path should be returned");
    let loaded = forge::infra::state::load_command_history(&root, 10).expect("history should load");

    assert!(path.ends_with(".forge/state/command-history.jsonl"));
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].raw, "pwd");
    assert_eq!(loaded[0].status, CommandStatus::Succeeded);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn shell_session_persists_as_toml() {
    let root = unique_temp_root();
    let mut session = ShellSessionState::new(
        std::env::current_dir().expect("current dir should exist"),
        "/bin/zsh".to_string(),
    );
    session.previous_cwd = Some(root.clone());
    session
        .env
        .insert("FORGE_MODE".to_string(), "local".to_string());
    session.last_exit_status = Some(7);

    let path =
        forge::infra::state::save_shell_session(&root, &session).expect("session should save");
    let loaded = forge::infra::state::load_shell_session(&root)
        .expect("session should load")
        .expect("session should exist");

    assert!(path.ends_with(".forge/state/session.toml"));
    assert_eq!(loaded.cwd, session.cwd);
    assert_eq!(loaded.previous_cwd, session.previous_cwd);
    assert_eq!(loaded.shell, "/bin/zsh");
    assert_eq!(
        loaded.env.get("FORGE_MODE").map(String::as_str),
        Some("local")
    );
    assert_eq!(loaded.last_exit_status, Some(7));

    let _ = std::fs::remove_dir_all(root);
}

fn test_store() -> AppStore {
    let config = ForgeConfig::default();
    let state = AppState::new(config, ProjectContext::default(), GitSnapshot::default());
    AppStore::new(state)
}

fn history_entry(raw: &str) -> CommandHistoryEntry {
    history_entry_with(
        None,
        raw,
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        false,
        classify_command(raw),
    )
}

fn history_entry_with(
    command_id: Option<CommandId>,
    raw: &str,
    cwd: std::path::PathBuf,
    background: bool,
    safety_class: SafetyClass,
) -> CommandHistoryEntry {
    let now = now_utc();
    CommandHistoryEntry {
        command_id,
        raw: raw.to_string(),
        cwd,
        shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()),
        provenance: CommandProvenance::UserInput,
        background,
        safety_class,
        status: CommandStatus::Succeeded,
        submitted_at: now,
        started_at: Some(now),
        ended_at: Some(now),
        exit_code: Some(0),
    }
}

fn unique_temp_root() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("forge-history-test-{}-{nanos}", std::process::id()))
}

fn buffer_rows(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
    let area = *buffer.area();
    (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect()
}

fn find_text(rows: &[String], needle: &str) -> Option<(u16, u16)> {
    rows.iter()
        .enumerate()
        .find_map(|(y, row)| row.find(needle).map(|x| (x as u16, y as u16)))
}

fn find_symbol(buffer: &ratatui::buffer::Buffer, symbol: &str) -> Option<(u16, u16)> {
    let area = *buffer.area();
    for y in 0..area.height {
        for x in 0..area.width {
            if buffer
                .cell((x, y))
                .is_some_and(|cell| cell.symbol() == symbol)
            {
                return Some((x, y));
            }
        }
    }
    None
}

fn find_symbol_on_row(
    buffer: &ratatui::buffer::Buffer,
    y: u16,
    symbol: &str,
) -> Option<(u16, u16)> {
    let area = *buffer.area();
    (0..area.width).find_map(|x| {
        buffer
            .cell((x, y))
            .filter(|cell| cell.symbol() == symbol)
            .map(|_| (x, y))
    })
}
