use std::path::{Path, PathBuf};
use std::process::Command;

use crate::domain::GitSnapshot;
use crate::shared::error::{AppResult, InfraError};
use crate::shared::time::now_utc;

pub fn snapshot_git(root: &Path) -> AppResult<GitSnapshot> {
    let branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let head = run_git(root, &["rev-parse", "--short", "HEAD"])?;
    let status = run_git(root, &["status", "--porcelain"])?;
    let changed_files = status
        .lines()
        .filter_map(|line| {
            line.get(3..)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    Ok(GitSnapshot {
        root: Some(root.to_path_buf()),
        branch: Some(branch.trim().to_string()),
        head: Some(head.trim().to_string()),
        is_dirty: !changed_files.is_empty(),
        changed_files,
        last_refreshed: now_utc(),
    })
}

fn run_git(root: &Path, args: &[&str]) -> AppResult<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| InfraError::Git {
            path: PathBuf::from(root),
            message: error.to_string(),
        })?;

    if !output.status.success() {
        return Err(InfraError::Git {
            path: PathBuf::from(root),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        }
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
