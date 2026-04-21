use std::collections::BTreeMap;
use std::time::SystemTime;

use crate::domain::{LogEntry, LogSeverity, LogSource, LogStream, OutputStream};
use crate::shared::ids::{CommandId, LogId, ServiceId};

pub fn normalize_command_output(
    id: LogId,
    ts: SystemTime,
    command_id: CommandId,
    service_id: Option<ServiceId>,
    stream: OutputStream,
    chunk: String,
) -> LogEntry {
    let severity = infer_severity(&chunk, stream);
    let stream_type = match stream {
        OutputStream::Stdout => LogStream::Stdout,
        OutputStream::Stderr => LogStream::Stderr,
    };

    let mut fields = BTreeMap::new();
    fields.insert("command_id".to_string(), command_id.to_string());
    if let Some(service_id) = service_id {
        fields.insert("service_id".to_string(), service_id.to_string());
    }

    LogEntry {
        id,
        ts,
        source: service_id
            .map(LogSource::Service)
            .unwrap_or(LogSource::Command(command_id)),
        stream: stream_type,
        service_id,
        command_id: Some(command_id),
        severity,
        raw: chunk,
        fields,
    }
}

pub fn infer_severity(chunk: &str, stream: OutputStream) -> LogSeverity {
    let lower = chunk.to_ascii_lowercase();
    if matches!(stream, OutputStream::Stderr) || lower.contains("error") || lower.contains("failed")
    {
        LogSeverity::Error
    } else if lower.contains("warn") {
        LogSeverity::Warn
    } else if lower.contains("debug") {
        LogSeverity::Debug
    } else if lower.contains("trace") {
        LogSeverity::Trace
    } else {
        LogSeverity::Info
    }
}

pub fn source_key(source: &LogSource) -> String {
    match source {
        LogSource::Command(command_id) => format!("command:{command_id}"),
        LogSource::Service(service_id) => format!("service:{service_id}"),
        LogSource::System => "system".to_string(),
        LogSource::Git => "git".to_string(),
        LogSource::Ai => "ai".to_string(),
    }
}
