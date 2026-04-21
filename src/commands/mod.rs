use crate::domain::{AiRequestKind, CommandProvenance};
use crate::shared::ids::CommandId;

#[derive(Debug, Clone)]
pub enum ParsedInput {
    Execute {
        command: String,
        background: bool,
        provenance: CommandProvenance,
    },
    Quit,
    Clear,
    NextTab,
    PrevTab,
    ApprovePending,
    DenyPending,
    Cancel(CommandId),
    Help,
    AiPrompt {
        prompt: String,
        kind: AiRequestKind,
    },
}

pub fn parse_input(input: &str) -> Result<ParsedInput, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("input is empty".to_string());
    }

    if !trimmed.starts_with('/') {
        let (command, background) = parse_shell(trimmed);
        return Ok(ParsedInput::Execute {
            command,
            background,
            provenance: CommandProvenance::UserInput,
        });
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

    Err(format!("unknown slash command: {trimmed}"))
}

fn parse_shell(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    if let Some(stripped) = trimmed.strip_suffix('&') {
        return (stripped.trim().to_string(), true);
    }
    (trimmed.to_string(), false)
}
