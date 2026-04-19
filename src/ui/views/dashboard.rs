use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, DashboardTab, ProcessStatus, ServiceHealth};

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = format!("Dashboard [{}]", state.ui.dashboard_tab.title());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    let lines = match state.ui.dashboard_tab {
        DashboardTab::Services => services_lines(state),
        DashboardTab::Processes => processes_lines(state),
        DashboardTab::Logs => log_lines(state),
        DashboardTab::Git => git_lines(state),
        DashboardTab::Tests => test_lines(state),
    };

    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn services_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from("Alt+Left/Alt+Right changes tabs.")];
    if state.services.registry.is_empty() {
        lines.push(Line::from("No managed services yet."));
        return lines;
    }

    for service in state.services.registry.iter().rev().take(12) {
        let health = match service.health {
            ServiceHealth::Healthy => "healthy",
            ServiceHealth::Starting => "starting",
            ServiceHealth::Degraded => "degraded",
            ServiceHealth::Unhealthy => "unhealthy",
            ServiceHealth::Stopped => "stopped",
            ServiceHealth::Unknown => "unknown",
        };
        lines.push(Line::from(format!(
            "#{} {} [{}] pid:{:?}",
            service.id, service.name, health, service.pid
        )));
    }
    lines
}

fn processes_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if state.processes.snapshots.is_empty() {
        lines.push(Line::from("No tracked processes."));
        return lines;
    }

    for process in state.processes.snapshots.iter().rev().take(12) {
        let status = match process.status {
            ProcessStatus::Starting => "starting",
            ProcessStatus::Running => "running",
            ProcessStatus::Exited => "exited",
            ProcessStatus::Failed => "failed",
            ProcessStatus::Cancelled => "cancelled",
        };
        lines.push(Line::from(format!(
            "pid:{:?} [{}] {}",
            process.pid, status, process.label
        )));
    }
    lines
}

fn log_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if state.logs.recent.is_empty() {
        lines.push(Line::from("No logs captured yet."));
        return lines;
    }

    for entry in state.logs.recent.iter().rev().take(14) {
        let level = match entry.severity {
            crate::domain::LogSeverity::Trace => "trace",
            crate::domain::LogSeverity::Debug => "debug",
            crate::domain::LogSeverity::Info => "info",
            crate::domain::LogSeverity::Warn => "warn",
            crate::domain::LogSeverity::Error => "error",
            crate::domain::LogSeverity::Unknown => "unknown",
        };
        lines.push(Line::from(format!("[{level}] {}", entry.raw)));
    }
    lines
}

fn git_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(format!(
        "Branch: {}",
        state.git.branch.clone().unwrap_or_else(|| "unknown".to_string())
    )));
    lines.push(Line::from(format!(
        "HEAD: {}",
        state.git.head.clone().unwrap_or_else(|| "unknown".to_string())
    )));
    lines.push(Line::from(format!(
        "Dirty: {}",
        if state.git.is_dirty { "yes" } else { "no" }
    )));
    if state.git.changed_files.is_empty() {
        lines.push(Line::from("Changed files: none"));
    } else {
        lines.push(Line::from("Changed files:"));
        for file in state.git.changed_files.iter().take(10) {
            lines.push(Line::from(format!(" - {file}")));
        }
    }
    lines
}

fn test_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if state.tests.recent_runs.is_empty() {
        lines.push(Line::from("No test runs observed yet."));
        return lines;
    }

    for run in state.tests.recent_runs.iter().rev().take(8) {
        lines.push(Line::from(format!(
            "{} [{:?}] pass:{} fail:{}",
            run.runner, run.status, run.pass_count, run.fail_count
        )));
    }
    lines
}
