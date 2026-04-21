use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use tracing_subscriber::EnvFilter;

use crate::config::ObservabilityConfig;
use crate::shared::error::AppResult;

pub struct ObservabilityHandle {
    pub log_file: Option<PathBuf>,
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

pub fn init_observability(
    config: &ObservabilityConfig,
    cwd: &Path,
) -> AppResult<ObservabilityHandle> {
    if config.log_to_file {
        let logs_dir = cwd.join(".forge").join("logs");
        fs::create_dir_all(&logs_dir)?;
        let log_file = logs_dir.join("forge.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;
        let (writer, guard) = tracing_appender::non_blocking(file);
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_new(config.log_level.clone())
                    .unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_ansi(false)
            .with_writer(writer)
            .finish();
        let _ = tracing::subscriber::set_global_default(subscriber);
        return Ok(ObservabilityHandle {
            log_file: Some(log_file),
            _guard: Some(guard),
        });
    }

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_new(config.log_level.clone()).unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(false)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    Ok(ObservabilityHandle {
        log_file: None,
        _guard: None,
    })
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crate::ui::terminal::restore_terminal();
        previous(info);
    }));
}
