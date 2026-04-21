use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone)]
pub struct GitSnapshot {
    pub root: Option<PathBuf>,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub is_dirty: bool,
    pub changed_files: Vec<String>,
    pub last_refreshed: SystemTime,
}

impl Default for GitSnapshot {
    fn default() -> Self {
        Self {
            root: None,
            branch: None,
            head: None,
            is_dirty: false,
            changed_files: Vec::new(),
            last_refreshed: SystemTime::now(),
        }
    }
}
