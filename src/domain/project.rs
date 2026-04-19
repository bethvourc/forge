use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub root: PathBuf,
    pub name: String,
    pub config_paths: Vec<PathBuf>,
    pub stack_hints: Vec<String>,
    pub discovered_files: Vec<PathBuf>,
}

impl Default for ProjectContext {
    fn default() -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "forge".to_string());

        Self {
            root,
            name,
            config_paths: Vec::new(),
            stack_hints: Vec::new(),
            discovered_files: Vec::new(),
        }
    }
}
