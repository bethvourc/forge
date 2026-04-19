use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicU64, Ordering};

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        pub struct $name(pub u64);

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(ApprovalId);
define_id!(CommandId);
define_id!(JobId);
define_id!(LogId);
define_id!(NotificationId);
define_id!(ProcessId);
define_id!(ServiceId);
define_id!(TimelineId);

#[derive(Debug, Default)]
pub struct IdGenerator {
    next: AtomicU64,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }

    pub fn next_raw(&self) -> u64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }

    pub fn next_command(&self) -> CommandId {
        CommandId(self.next_raw())
    }

    pub fn next_job(&self) -> JobId {
        JobId(self.next_raw())
    }

    pub fn next_service(&self) -> ServiceId {
        ServiceId(self.next_raw())
    }

    pub fn next_log(&self) -> LogId {
        LogId(self.next_raw())
    }

    pub fn next_timeline(&self) -> TimelineId {
        TimelineId(self.next_raw())
    }

    pub fn next_approval(&self) -> ApprovalId {
        ApprovalId(self.next_raw())
    }

    pub fn next_process(&self) -> ProcessId {
        ProcessId(self.next_raw())
    }

    pub fn next_notification(&self) -> NotificationId {
        NotificationId(self.next_raw())
    }
}

