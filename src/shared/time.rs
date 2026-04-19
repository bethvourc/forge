use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn now_utc() -> SystemTime {
    SystemTime::now()
}

pub fn format_timestamp(ts: SystemTime) -> String {
    let elapsed = ts
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0));
    let secs = elapsed.as_secs();
    let day_secs = secs % 86_400;
    let hours = day_secs / 3_600;
    let minutes = (day_secs % 3_600) / 60;
    let seconds = day_secs % 60;
    let millis = elapsed.subsec_millis();
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}
