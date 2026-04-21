use std::time::SystemTime;

use crate::shared::ids::TimelineId;
use crate::shared::ring_buffer::RingBuffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineKind {
    System,
    Ai,
    Command,
    Log,
    Approval,
    Error,
}

#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub id: TimelineId,
    pub at: SystemTime,
    pub kind: TimelineKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct TimelineState {
    pub entries: RingBuffer<TimelineEntry>,
}

impl TimelineState {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: RingBuffer::new(capacity),
        }
    }
}
