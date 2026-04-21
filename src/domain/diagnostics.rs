use std::path::PathBuf;
use std::time::SystemTime;

use crate::shared::ring_buffer::RingBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct DiagnosticRecord {
    pub at: SystemTime,
    pub level: DiagnosticLevel,
    pub message: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticsState {
    pub records: RingBuffer<DiagnosticRecord>,
    pub log_file: Option<PathBuf>,
}

impl DiagnosticsState {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: RingBuffer::new(capacity),
            log_file: None,
        }
    }
}
