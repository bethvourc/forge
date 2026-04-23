use crate::domain::{
    AiCommandContext, AiContextBundle, AiGitContext, AiLogContext, AiProjectContext,
    AiServiceContext, AiShellSessionContext, AiTestContext, AiTimelineContext, AiUiContext,
    AppState, FocusTarget, LogSource, ModalState,
};

pub fn build_context(state: &AppState) -> AiContextBundle {
    let limits = &state.config.ai;

    AiContextBundle {
        ui: AiUiContext {
            focus: focus_label(state.ui.focus).to_string(),
            dashboard_tab: state.ui.dashboard_tab.title().to_string(),
            modal: state.ui.modal.as_ref().map(modal_label),
            pending_approvals: state.approvals.pending.len(),
        },
        project: AiProjectContext {
            name: state.project.name.clone(),
            root: state.project.root.clone(),
            stack_hints: state.project.stack_hints.clone(),
            config_paths: state.project.config_paths.clone(),
            discovered_files: state
                .project
                .discovered_files
                .iter()
                .take(limits.max_changed_files)
                .cloned()
                .collect(),
        },
        git: AiGitContext {
            branch: state.git.branch.clone(),
            head: state.git.head.clone(),
            is_dirty: state.git.is_dirty,
            changed_files: state
                .git
                .changed_files
                .iter()
                .take(limits.max_changed_files)
                .cloned()
                .collect(),
        },
        session: AiShellSessionContext {
            cwd: state.commands.session.cwd.clone(),
            shell: state.commands.session.shell.clone(),
            env_overlay_count: state.commands.session.env.len(),
            last_exit_status: state.commands.session.last_exit_status,
        },
        commands: state
            .commands
            .records
            .iter()
            .rev()
            .take(limits.max_recent_commands)
            .map(|record| AiCommandContext {
                id: record.id,
                raw: record.raw.clone(),
                cwd: record.cwd.clone(),
                provenance: record.provenance,
                status: record.status,
                exit_code: record.exit_code,
                output_line_count: record.output_line_count,
                background: record.background,
            })
            .collect(),
        services: state
            .services
            .registry
            .iter()
            .rev()
            .take(limits.max_recent_services)
            .map(|service| AiServiceContext {
                id: service.id,
                name: service.name.clone(),
                health: service.health,
                pid: service.pid,
                ports: service.ports.clone(),
                tags: service.tags.clone(),
            })
            .collect(),
        tests: state
            .tests
            .recent_runs
            .iter()
            .rev()
            .take(limits.max_recent_tests)
            .map(|run| AiTestContext {
                runner: run.runner.clone(),
                status: run.status,
                pass_count: run.pass_count,
                fail_count: run.fail_count,
                failed_tests: run.failed_tests.clone(),
            })
            .collect(),
        logs: state
            .logs
            .recent
            .iter()
            .rev()
            .take(limits.max_recent_logs)
            .map(|entry| AiLogContext {
                ts: entry.ts,
                source_label: log_source_label(&entry.source),
                stream: entry.stream,
                severity: entry.severity,
                service_id: entry.service_id,
                command_id: entry.command_id,
                raw: entry.raw.clone(),
            })
            .collect(),
        timeline: state
            .timeline
            .entries
            .iter()
            .rev()
            .take(limits.max_recent_events)
            .map(|entry| AiTimelineContext {
                at: entry.at,
                kind: entry.kind,
                message: entry.message.clone(),
            })
            .collect(),
    }
}

fn focus_label(focus: FocusTarget) -> &'static str {
    match focus {
        FocusTarget::CommandPane => "command",
        FocusTarget::DashboardPane => "dashboard",
        FocusTarget::EventStream => "events",
        FocusTarget::Modal => "modal",
    }
}

fn modal_label(modal: &ModalState) -> String {
    match modal {
        ModalState::Approval(id) => format!("approval:{id}"),
        ModalState::Help => "help".to_string(),
        ModalState::History => "history".to_string(),
        ModalState::Error(_) => "error".to_string(),
    }
}

fn log_source_label(source: &LogSource) -> String {
    match source {
        LogSource::Command(command_id) => format!("command:{command_id}"),
        LogSource::Service(service_id) => format!("service:{service_id}"),
        LogSource::System => "system".to_string(),
        LogSource::Git => "git".to_string(),
        LogSource::Ai => "ai".to_string(),
    }
}
