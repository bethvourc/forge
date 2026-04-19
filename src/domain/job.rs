use std::time::SystemTime;

use crate::shared::ids::{CommandId, JobId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub id: JobId,
    pub command_id: CommandId,
    pub label: String,
    pub pid: Option<u32>,
    pub background: bool,
    pub status: JobStatus,
    pub started_at: Option<SystemTime>,
    pub ended_at: Option<SystemTime>,
}

#[derive(Debug, Clone, Default)]
pub struct JobState {
    pub records: Vec<JobRecord>,
}

