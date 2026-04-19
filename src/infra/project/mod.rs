use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::ProjectContext;
use crate::shared::error::{AppResult, InfraError};

pub fn scan_project(
    start_dir: &Path,
    global_config_path: Option<PathBuf>,
    project_config_path: Option<PathBuf>,
) -> AppResult<ProjectContext> {
    let root = find_project_root(start_dir).ok_or_else(|| InfraError::Project {
        path: start_dir.to_path_buf(),
        message: "could not determine project root".to_string(),
    })?;
    let name = root
        .file_name()
        .map(|segment| segment.to_string_lossy().into_owned())
        .unwrap_or_else(|| "forge".to_string());
    let mut discovered_files = Vec::new();
    let mut stack_hints = Vec::new();
    let mut config_paths = Vec::new();

    for candidate in [
        root.join("Cargo.toml"),
        root.join("docker-compose.yml"),
        root.join("docker-compose.yaml"),
        root.join(".forge").join("config.toml"),
        root.join("package.json"),
    ] {
        if candidate.exists() {
            if candidate
                .file_name()
                .is_some_and(|name| name == "Cargo.toml")
            {
                stack_hints.push("rust".to_string());
            }
            if candidate
                .file_name()
                .is_some_and(|name| name.to_string_lossy().contains("docker-compose"))
            {
                stack_hints.push("docker-compose".to_string());
            }
            discovered_files.push(candidate);
        }
    }

    if let Some(path) = global_config_path {
        config_paths.push(path);
    }
    if let Some(path) = project_config_path {
        config_paths.push(path);
    }
    if config_paths.is_empty() {
        let candidate = root.join(".forge").join("config.toml");
        if candidate.exists() {
            config_paths.push(candidate);
        }
    }

    let top_entries = fs::read_dir(&root)
        .map_err(|source| InfraError::Project {
            path: root.clone(),
            message: source.to_string(),
        })?
        .flatten()
        .map(|entry| entry.path())
        .take(10)
        .collect::<Vec<_>>();
    discovered_files.extend(top_entries);

    Ok(ProjectContext {
        root,
        name,
        config_paths,
        stack_hints,
        discovered_files,
    })
}

fn find_project_root(start_dir: &Path) -> Option<PathBuf> {
    for candidate in start_dir.ancestors() {
        if candidate.join(".git").exists() {
            return Some(candidate.to_path_buf());
        }
    }
    Some(start_dir.to_path_buf())
}
