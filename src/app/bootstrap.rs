use crate::ai::AiRuntime;
use crate::app::AppStore;
use crate::config::{load_config, CliArgs};
use crate::domain::{
    AppState, DiagnosticLevel, DiagnosticRecord, GitSnapshot, ProjectContext, TimelineKind,
};
use crate::infra::{git, project, state as local_state};
use crate::observability::{init_observability, install_panic_hook};
use crate::runtime::Runtime;
use crate::shared::error::AppResult;
use crate::shared::time::now_utc;
use crate::ui::terminal::TerminalUi;

pub async fn bootstrap(cli: CliArgs) -> AppResult<Runtime> {
    let loaded = load_config(&cli)?;
    let observability = init_observability(&loaded.config.observability, &loaded.cwd)?;
    install_panic_hook();

    let project = project::scan_project(
        &loaded.cwd,
        loaded.global_config_path.clone(),
        loaded.project_config_path.clone(),
    )
    .unwrap_or_else(|_| ProjectContext::default());
    let git = if loaded.config.git.enabled {
        git::snapshot_git(&project.root).unwrap_or_else(|_| GitSnapshot::default())
    } else {
        GitSnapshot::default()
    };

    let mut state = AppState::new(loaded.config.clone(), project, git);
    match local_state::load_command_history(
        &state.project.root,
        state.config.commands.history_limit,
    ) {
        Ok(history) => {
            state.commands.history = history;
        }
        Err(error) => {
            state.diagnostics.records.push(DiagnosticRecord {
                at: now_utc(),
                level: DiagnosticLevel::Warn,
                message: format!("failed to load command history: {error}"),
                context: Some(state.project.root.display().to_string()),
            });
        }
    }
    state.diagnostics.log_file = observability.log_file.clone();
    state.timeline.entries.push(crate::domain::TimelineEntry {
        id: crate::shared::ids::TimelineId(0),
        at: now_utc(),
        kind: TimelineKind::System,
        message: "Forge bootstrapped".to_string(),
    });

    let store = AppStore::new(state);
    let terminal = TerminalUi::new()?;
    let ai_runtime = AiRuntime::from_config(&loaded.config.ai);
    Ok(Runtime::new(store, terminal, observability, ai_runtime))
}
