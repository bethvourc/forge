use std::time::SystemTime;

use crate::domain::command::ExecutionRequest;
use crate::shared::ids::{ApprovalId, CommandId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SafetyClass {
    Passive,
    Safe,
    Caution,
    Risky,
    Destructive,
}

impl SafetyClass {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Passive => "Passive",
            Self::Safe => "Safe",
            Self::Caution => "Caution",
            Self::Risky => "Risky",
            Self::Destructive => "Destructive",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalMode {
    InlineReview,
    ModalConfirm,
}

#[derive(Debug, Clone)]
pub struct ApprovalRequest {
    pub id: ApprovalId,
    pub command_id: CommandId,
    pub created_at: SystemTime,
    pub class: SafetyClass,
    pub mode: ApprovalMode,
    pub summary: String,
    pub detail: String,
    pub execution: ExecutionRequest,
}

#[derive(Debug, Clone)]
pub struct ApprovalDecision {
    pub approval_id: ApprovalId,
    pub command_id: CommandId,
    pub approved: bool,
    pub decided_at: SystemTime,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ApprovalState {
    pub pending: Vec<ApprovalRequest>,
    pub history: Vec<ApprovalDecision>,
}
