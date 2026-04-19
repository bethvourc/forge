use crate::config::SafetyConfig;
use crate::domain::{ApprovalMode, ApprovalRequest, SafetyClass};

pub fn classify_command(command: &str) -> SafetyClass {
    let normalized = command.trim().to_ascii_lowercase();

    if normalized.is_empty() {
        return SafetyClass::Passive;
    }

    let destructive_patterns = [
        "rm -rf",
        "git reset --hard",
        "git clean -fd",
        "mkfs",
        "shutdown",
        "reboot",
        "docker system prune",
    ];
    if destructive_patterns
        .iter()
        .any(|pattern| normalized.contains(pattern))
    {
        return SafetyClass::Destructive;
    }

    let risky_patterns = [
        "git reset",
        "git clean",
        "sed -i",
        "cargo add",
        "cargo remove",
        "npm install",
        "pnpm add",
        "pip install",
        "mv ",
        "cp ",
        " >",
        " >>",
        "tee ",
    ];
    if risky_patterns
        .iter()
        .any(|pattern| normalized.contains(pattern))
    {
        return SafetyClass::Risky;
    }

    let caution_patterns = [
        "kill ",
        "pkill",
        "docker restart",
        "docker compose restart",
        "brew services restart",
        "cargo test",
        "cargo fmt",
        "cargo clippy",
    ];
    if caution_patterns
        .iter()
        .any(|pattern| normalized.contains(pattern))
    {
        return SafetyClass::Caution;
    }

    let passive_prefixes = [
        "ls",
        "pwd",
        "echo",
        "cat",
        "git status",
        "git diff",
        "ps",
        "top",
        "tail",
        "head",
    ];
    if passive_prefixes
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return SafetyClass::Passive;
    }

    SafetyClass::Safe
}

pub fn approval_mode_for(class: SafetyClass, config: &SafetyConfig) -> Option<ApprovalMode> {
    match class {
        SafetyClass::Passive | SafetyClass::Safe => None,
        SafetyClass::Caution if config.caution_requires_review => Some(ApprovalMode::InlineReview),
        SafetyClass::Caution => None,
        SafetyClass::Risky => Some(ApprovalMode::ModalConfirm),
        SafetyClass::Destructive if config.destructive_requires_confirmation => {
            Some(ApprovalMode::ModalConfirm)
        }
        SafetyClass::Destructive => None,
    }
}

pub fn approval_message(request: &ApprovalRequest) -> String {
    match request.class {
        SafetyClass::Caution => format!("Review before running: {}", request.summary),
        SafetyClass::Risky => format!("Approval required: {}", request.summary),
        SafetyClass::Destructive => format!("Destructive action requires confirmation: {}", request.summary),
        SafetyClass::Passive | SafetyClass::Safe => request.summary.clone(),
    }
}

