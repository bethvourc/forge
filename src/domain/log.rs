use std::collections::BTreeMap;
use std::time::SystemTime;

use crate::shared::ids::{CommandId, LogId, ServiceId};
use crate::shared::ring_buffer::RingBuffer;

#[derive(Debug, Clone)]
pub enum LogSource {
    Command(CommandId),
    Service(ServiceId),
    System,
    Git,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub id: LogId,
    pub ts: SystemTime,
    pub source: LogSource,
    pub stream: LogStream,
    pub service_id: Option<ServiceId>,
    pub command_id: Option<CommandId>,
    pub severity: LogSeverity,
    pub raw: String,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct LogFilters {
    pub source_key: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogState {
    pub recent: RingBuffer<LogEntry>,
    pub per_source: BTreeMap<String, RingBuffer<LogEntry>>,
    pub filters: LogFilters,
}

impl LogState {
    pub fn new(global_capacity: usize) -> Self {
        Self {
            recent: RingBuffer::new(global_capacity),
            per_source: BTreeMap::new(),
            filters: LogFilters::default(),
        }
    }
}

