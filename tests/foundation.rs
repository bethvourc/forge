use std::collections::BTreeMap;
use std::time::SystemTime;

use forge::app::{AppAction, AppStore, Effect};
use forge::commands::{parse_input, slash_command_suggestions, ParsedInput};
use forge::config::ForgeConfig;
use forge::domain::{
    AiActionProposal, AiRequestKind, AiResponse, AppState, ApprovalMode, CommandProvenance,
    CommandStatus, DashboardTab, FocusTarget, GitSnapshot, InputMode, LogEntry, LogSeverity,
    LogSource, LogStream, ModalState, ProjectContext, SafetyClass,
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
fn parses_supported_slash_commands() {
    assert!(matches!(parse_input("/quit").unwrap(), ParsedInput::Quit));
    assert!(matches!(parse_input("/exit").unwrap(), ParsedInput::Quit));
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
    store.state_mut().commands.history = vec!["pwd".to_string(), "cargo test".to_string()];
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

fn test_store() -> AppStore {
    let config = ForgeConfig::default();
    let state = AppState::new(config, ProjectContext::default(), GitSnapshot::default());
    AppStore::new(state)
}
