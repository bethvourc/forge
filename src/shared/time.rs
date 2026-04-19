use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now_utc() -> SystemTime {
    SystemTime::now()
}

pub fn format_timestamp(ts: SystemTime) -> String {
    let elapsed = ts
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let secs = elapsed.as_secs();
    let millis = elapsed.subsec_millis();
    format!("{secs}.{millis:03}")
}

