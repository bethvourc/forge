use std::time::SystemTime;

use crate::shared::ids::{CommandId, JobId, ProcessId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessStatus {
    Starting,
    Running,
    Exited,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ProcessSnapshot {
    pub id: ProcessId,
    pub command_id: Option<CommandId>,
    pub job_id: Option<JobId>,
    pub pid: Option<u32>,
    pub label: String,
    pub status: ProcessStatus,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
    pub observed_at: SystemTime,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessState {
    pub snapshots: Vec<ProcessSnapshot>,
}

