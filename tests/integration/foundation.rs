use forge::commands::{parse_input, ParsedInput};
use forge::domain::CommandProvenance;
use forge::domain::SafetyClass;
use forge::safety::classify_command;

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
fn parses_cancel_slash_command() {
    let parsed = parse_input("/cancel 42").expect("cancel slash command should parse");
    match parsed {
        ParsedInput::Cancel(id) => assert_eq!(id.0, 42),
        other => panic!("unexpected parse result: {:?}", other),
    }
}

#[test]
fn safety_classifier_marks_destructive_commands() {
    let class = classify_command("rm -rf build");
    assert!(matches!(class, SafetyClass::Destructive));
}
