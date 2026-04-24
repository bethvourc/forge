use crate::domain::{AiRequestKind, CommandProvenance};
use crate::shared::ids::CommandId;

#[derive(Debug, Clone, Copy)]
pub struct SlashCommandSpec {
    pub usage: &'static str,
    pub completion: &'static str,
    pub summary: &'static str,
}

impl SlashCommandSpec {
    pub fn accepts_arguments(&self) -> bool {
        self.completion.ends_with(' ')
    }
}

const SLASH_COMMANDS: [SlashCommandSpec; 15] = [
    SlashCommandSpec {
        usage: "/help",
        completion: "/help",
        summary: "Open the Forge command reference.",
    },
    SlashCommandSpec {
        usage: "/ai <prompt>",
        completion: "/ai ",
        summary: "Ask Forge for grounded assistance.",
    },
    SlashCommandSpec {
        usage: "/diagnose <prompt>",
        completion: "/diagnose ",
        summary: "Request a diagnosis tied to live runtime context.",
    },
    SlashCommandSpec {
        usage: "/apply <n>",
        completion: "/apply ",
        summary: "Run AI proposal n through safety checks.",
    },
    SlashCommandSpec {
        usage: "/bg <cmd>",
        completion: "/bg ",
        summary: "Queue a background command or long-lived process.",
    },
    SlashCommandSpec {
        usage: "/cancel <id>",
        completion: "/cancel ",
        summary: "Cancel a tracked managed command.",
    },
    SlashCommandSpec {
        usage: "/history",
        completion: "/history",
        summary: "Open recent shell command history.",
    },
    SlashCommandSpec {
        usage: "/rerun <id>",
        completion: "/rerun ",
        summary: "Replay a tracked command through safety checks.",
    },
    SlashCommandSpec {
        usage: "/rerun-last",
        completion: "/rerun-last",
        summary: "Replay the latest shell history entry.",
    },
    SlashCommandSpec {
        usage: "/approve",
        completion: "/approve",
        summary: "Approve the current pending action.",
    },
    SlashCommandSpec {
        usage: "/deny",
        completion: "/deny",
        summary: "Dismiss the current pending action.",
    },
    SlashCommandSpec {
        usage: "/tab next",
        completion: "/tab next",
        summary: "Move the dashboard to the next tab.",
    },
    SlashCommandSpec {
        usage: "/tab prev",
        completion: "/tab prev",
        summary: "Move the dashboard to the previous tab.",
    },
    SlashCommandSpec {
        usage: "/clear",
        completion: "/clear",
        summary: "Clear the visible input and log buffer.",
    },
    SlashCommandSpec {
        usage: "/quit",
        completion: "/quit",
        summary: "Exit Forge cleanly.",
    },
];

#[derive(Debug, Clone)]
pub enum ParsedInput {
    Execute {
        command: String,
        background: bool,
        provenance: CommandProvenance,
    },
    ChangeDirectory {
        target: Option<String>,
    },
    SetEnv {
        key: String,
        value: String,
    },
    UnsetEnv {
        key: String,
    },
    Quit,
    Clear,
    NextTab,
    PrevTab,
    ApprovePending,
    DenyPending,
    Cancel(CommandId),
    History,
    Rerun(CommandId),
    RerunLast,
    Help,
    AiPrompt {
        prompt: String,
        kind: AiRequestKind,
    },
    ApplyAiProposal(usize),
}

pub fn slash_command_catalog() -> &'static [SlashCommandSpec] {
    &SLASH_COMMANDS
}

pub fn slash_command_suggestions(input: &str) -> Vec<&'static SlashCommandSpec> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return Vec::new();
    }

    if trimmed == "/" {
        return SLASH_COMMANDS.iter().collect();
    }

    let normalized = trimmed.to_ascii_lowercase();
    SLASH_COMMANDS
        .iter()
        .filter(|spec| {
            spec.usage.to_ascii_lowercase().starts_with(&normalized)
                || spec
                    .completion
                    .trim_end()
                    .to_ascii_lowercase()
                    .starts_with(&normalized)
        })
        .collect()
}

pub fn slash_palette_active(input: &str) -> bool {
    input.trim_start().starts_with('/') && !slash_command_suggestions(input).is_empty()
}

pub fn parse_input(input: &str) -> Result<ParsedInput, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("input is empty".to_string());
    }

    if trimmed == "clear" {
        return Ok(ParsedInput::Clear);
    }
    if trimmed == "exit" || trimmed == "quit" {
        return Ok(ParsedInput::Quit);
    }
    if let Some(target) = parse_cd_builtin(trimmed) {
        return Ok(ParsedInput::ChangeDirectory { target });
    }
    if let Some((key, value)) = parse_export_builtin(trimmed) {
        return Ok(ParsedInput::SetEnv { key, value });
    }
    if let Some(key) = parse_unset_builtin(trimmed) {
        return Ok(ParsedInput::UnsetEnv { key });
    }

    if !trimmed.starts_with('/') {
        return parse_shell_input(trimmed, CommandProvenance::UserInput);
    }

    if trimmed == "/quit" || trimmed == "/exit" {
        return Ok(ParsedInput::Quit);
    }
    if trimmed == "/clear" {
        return Ok(ParsedInput::Clear);
    }
    if trimmed == "/tab next" || trimmed == "/next-tab" {
        return Ok(ParsedInput::NextTab);
    }
    if trimmed == "/tab prev" || trimmed == "/prev-tab" {
        return Ok(ParsedInput::PrevTab);
    }
    if trimmed == "/approve" {
        return Ok(ParsedInput::ApprovePending);
    }
    if trimmed == "/deny" {
        return Ok(ParsedInput::DenyPending);
    }
    if trimmed == "/help" {
        return Ok(ParsedInput::Help);
    }
    if trimmed == "/history" {
        return Ok(ParsedInput::History);
    }
    if trimmed == "/rerun-last" {
        return Ok(ParsedInput::RerunLast);
    }
    if let Some(rest) = trimmed.strip_prefix("/bg ") {
        return Ok(ParsedInput::Execute {
            command: rest.trim().to_string(),
            background: true,
            provenance: CommandProvenance::SlashCommand,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("/cancel ") {
        let id = rest
            .trim()
            .parse::<u64>()
            .map_err(|_| "cancel expects a numeric command id".to_string())?;
        return Ok(ParsedInput::Cancel(CommandId(id)));
    }
    if let Some(rest) = trimmed.strip_prefix("/rerun ") {
        let id = rest
            .trim()
            .parse::<u64>()
            .map_err(|_| "rerun expects a numeric command id".to_string())?;
        return Ok(ParsedInput::Rerun(CommandId(id)));
    }
    if let Some(rest) = trimmed.strip_prefix("/ai ") {
        return Ok(ParsedInput::AiPrompt {
            prompt: rest.trim().to_string(),
            kind: AiRequestKind::Assist,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("/diagnose ") {
        return Ok(ParsedInput::AiPrompt {
            prompt: rest.trim().to_string(),
            kind: AiRequestKind::Diagnose,
        });
    }
    if let Some(rest) = trimmed.strip_prefix("/apply ") {
        let index = rest
            .trim()
            .parse::<usize>()
            .map_err(|_| "apply expects a numeric proposal index".to_string())?;
        if index == 0 {
            return Err("apply expects a proposal index starting at 1".to_string());
        }
        return Ok(ParsedInput::ApplyAiProposal(index));
    }
    if let Some(rest) = trimmed.strip_prefix("/run-proposal ") {
        let index = rest
            .trim()
            .parse::<usize>()
            .map_err(|_| "run-proposal expects a numeric proposal index".to_string())?;
        if index == 0 {
            return Err("run-proposal expects a proposal index starting at 1".to_string());
        }
        return Ok(ParsedInput::ApplyAiProposal(index));
    }

    Err(format!("unknown slash command: {trimmed}"))
}

pub fn parse_shell_input(
    input: &str,
    provenance: CommandProvenance,
) -> Result<ParsedInput, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("input is empty".to_string());
    }

    let (command, background) = parse_shell(trimmed);
    Ok(ParsedInput::Execute {
        command,
        background,
        provenance,
    })
}

fn parse_shell(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    if let Some(stripped) = trimmed.strip_suffix('&') {
        return (stripped.trim().to_string(), true);
    }
    (trimmed.to_string(), false)
}

fn parse_cd_builtin(input: &str) -> Option<Option<String>> {
    if input == "cd" {
        return Some(None);
    }

    let target = input.strip_prefix("cd ")?;
    let target = target.trim();
    if target_contains_shell_operator(target) {
        return None;
    }

    Some((!target.is_empty()).then(|| strip_matching_quotes(target).to_string()))
}

fn parse_export_builtin(input: &str) -> Option<(String, String)> {
    let assignment = input.strip_prefix("export ")?;
    let (key, value) = assignment.trim().split_once('=')?;
    let key = key.trim();
    if !is_valid_env_key(key) || value_contains_shell_evaluation(value) {
        return None;
    }

    Some((
        key.to_string(),
        strip_matching_quotes(value.trim()).to_string(),
    ))
}

fn parse_unset_builtin(input: &str) -> Option<String> {
    let key = input.strip_prefix("unset ")?.trim();
    if !is_valid_env_key(key) {
        return None;
    }

    Some(key.to_string())
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }

    chars.all(|char| char == '_' || char.is_ascii_alphanumeric())
}

fn value_contains_shell_evaluation(value: &str) -> bool {
    ["`", "$("].iter().any(|operator| value.contains(operator))
}

fn target_contains_shell_operator(target: &str) -> bool {
    ["&&", "||", ";", "|", ">", "<", "`", "$("]
        .iter()
        .any(|operator| target.contains(operator))
}

fn strip_matching_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes.first() == Some(&b'"') && bytes.last() == Some(&b'"'))
            || (bytes.first() == Some(&b'\'') && bytes.last() == Some(&b'\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}
