use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use tracing::warn;

use crate::domain::CommandHistoryEntry;
use crate::shared::error::AppResult;

const COMMAND_HISTORY_FILE: &str = "command-history.jsonl";

pub fn workspace_state_dir(project_root: &Path) -> PathBuf {
    project_root.join(".forge").join("state")
}

pub fn command_history_path(project_root: &Path) -> PathBuf {
    workspace_state_dir(project_root).join(COMMAND_HISTORY_FILE)
}

pub fn load_command_history(
    project_root: &Path,
    limit: usize,
) -> AppResult<Vec<CommandHistoryEntry>> {
    let path = command_history_path(project_root);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let mut entries = Vec::new();

    for (line_number, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match serde_json::from_str::<CommandHistoryEntry>(trimmed) {
            Ok(entry) => entries.push(entry),
            Err(error) => {
                warn!(
                    path = %path.display(),
                    line = line_number + 1,
                    %error,
                    "skipping invalid command history entry"
                );
            }
        }
    }

    trim_history(&mut entries, limit);
    Ok(entries)
}

pub fn save_command_history(
    project_root: &Path,
    entries: &[CommandHistoryEntry],
) -> AppResult<Option<PathBuf>> {
    if entries.is_empty() {
        return Ok(None);
    }

    let path = command_history_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp_path = path.with_extension("jsonl.tmp");
    {
        let file = File::create(&tmp_path)?;
        let mut writer = BufWriter::new(file);
        for entry in entries {
            serde_json::to_writer(&mut writer, entry)?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
    }

    fs::rename(&tmp_path, &path)?;
    Ok(Some(path))
}

fn trim_history(entries: &mut Vec<CommandHistoryEntry>, limit: usize) {
    if entries.len() > limit {
        let overflow = entries.len() - limit;
        entries.drain(0..overflow);
    }
}
