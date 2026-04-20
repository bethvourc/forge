use crate::ai::provider::{AiFuture, AiProvider};
use crate::domain::{
    AiActionProposal, AiCitation, AiRequest, AiRequestKind, AiResponse, SafetyClass,
};
use crate::shared::time::now_utc;

pub struct MockProvider {
    model: Option<String>,
}

impl MockProvider {
    pub fn new(model: Option<String>) -> Self {
        Self { model }
    }
}

impl AiProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn execute(&self, request: AiRequest) -> AiFuture {
        let model = self.model.clone();
        Box::pin(async move { Ok(mock_response(request, model)) })
    }
}

fn mock_response(request: AiRequest, model: Option<String>) -> AiResponse {
    let branch = request
        .context
        .git
        .branch
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let message = match request.kind {
        AiRequestKind::Assist => format!(
            "Mock provider reviewed the current Forge context for `{}` in repo `{}` on branch `{}`.",
            request.prompt, request.context.project.name, branch
        ),
        AiRequestKind::Diagnose => format!(
            "Mock diagnosis reviewed {} commands, {} logs, and {} timeline entries for `{}`.",
            request.context.commands.len(),
            request.context.logs.len(),
            request.context.timeline.len(),
            request.prompt
        ),
    };

    let mut recommendations = vec![
        format!("Review branch `{branch}` before applying changes."),
        format!(
            "Inspect the {} most recent timeline events for correlated state changes.",
            request.context.timeline.len()
        ),
    ];

    if let Some(log) = request.context.logs.first() {
        recommendations.push(format!(
            "Inspect `{}` for the newest log activity.",
            log.source_label
        ));
    }

    let proposals = match request.kind {
        AiRequestKind::Assist => vec![
            AiActionProposal {
                summary: "Inspect the current repository state".to_string(),
                detail: "Check the working tree before taking additional action.".to_string(),
                command: Some("git status".to_string()),
                safety_class: SafetyClass::Passive,
            },
            AiActionProposal {
                summary: "Run the Rust test suite".to_string(),
                detail: "Confirm the current behavior against the latest local changes."
                    .to_string(),
                command: Some("cargo test".to_string()),
                safety_class: SafetyClass::Caution,
            },
        ],
        AiRequestKind::Diagnose => vec![
            AiActionProposal {
                summary: "Re-run tests to reproduce the failure".to_string(),
                detail: "Use a fresh test run so the latest output is captured in Forge."
                    .to_string(),
                command: Some("cargo test".to_string()),
                safety_class: SafetyClass::Caution,
            },
            AiActionProposal {
                summary: "Review the current diff".to_string(),
                detail:
                    "Inspect the working tree for recent changes that correlate with the failure."
                        .to_string(),
                command: Some("git diff --stat".to_string()),
                safety_class: SafetyClass::Passive,
            },
        ],
    };

    AiResponse {
        request_id: request.id,
        created_at: now_utc(),
        provider: "mock".to_string(),
        model,
        summary: format!("Mock AI response for `{}`", request.prompt),
        message,
        recommendations,
        proposals,
        citations: vec![AiCitation {
            label: "context".to_string(),
            detail: format!(
                "repo={} dirty={} services={} tests={}",
                request.context.project.name,
                request.context.git.is_dirty,
                request.context.services.len(),
                request.context.tests.len()
            ),
        }],
    }
}
