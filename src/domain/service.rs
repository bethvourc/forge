use std::time::SystemTime;

use crate::shared::ids::{CommandId, ServiceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceSource {
    Config,
    ManagedCommand,
    ProcessObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceHealth {
    Unknown,
    Starting,
    Healthy,
    Degraded,
    Unhealthy,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct ServiceRecord {
    pub id: ServiceId,
    pub name: String,
    pub source: ServiceSource,
    pub pid: Option<u32>,
    pub ports: Vec<u16>,
    pub health: ServiceHealth,
    pub tags: Vec<String>,
    pub linked_command: Option<CommandId>,
    pub last_seen: SystemTime,
}

#[derive(Debug, Clone, Default)]
pub struct ServiceState {
    pub registry: Vec<ServiceRecord>,
}
