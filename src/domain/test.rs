use std::time::SystemTime;

use crate::shared::ids::CommandId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Running,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct TestRunRecord {
    pub runner: String,
    pub status: TestStatus,
    pub pass_count: usize,
    pub fail_count: usize,
    pub failed_tests: Vec<String>,
    pub duration_ms: Option<u128>,
    pub source_command: Option<CommandId>,
    pub recorded_at: SystemTime,
}

#[derive(Debug, Clone, Default)]
pub struct TestState {
    pub recent_runs: Vec<TestRunRecord>,
}
