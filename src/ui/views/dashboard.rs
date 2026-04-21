use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, DashboardTab, ProcessStatus};
use crate::shared::time::format_timestamp;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let block = Block::default()
        .title(Span::styled(" Dashboard ", theme::panel_title(focused)))
        .borders(Borders::ALL)
        .border_style(theme::pane_border(focused))
        .style(theme::primary().bg(theme::BG_BASE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(8),
        ])
        .split(inner);

    render_tabs(frame, sections[0], state);
    render_context(frame, sections[1], state);

    let lines = match state.ui.dashboard_tab {
        DashboardTab::Services => services_lines(state),
        DashboardTab::Processes => processes_lines(state),
        DashboardTab::Logs => log_lines(state),
        DashboardTab::Git => git_lines(state),
        DashboardTab::Tests => test_lines(state),
    };

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default())
            .wrap(Wrap { trim: false }),
        sections[2],
    );
}

fn services_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Managed services", theme::section_title()),
        Span::styled("  background jobs and long-lived processes", theme::muted()),
    ])];
    if state.services.registry.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No managed services yet.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![
            Span::styled("Try ", theme::muted()),
            Span::styled("sleep 30 &", theme::accent()),
            Span::styled(" or ", theme::muted()),
            Span::styled("/bg cargo run", theme::accent()),
            Span::styled(" to create one.", theme::muted()),
        ]));
        return lines;
    }

    for service in state.services.registry.iter().rev().take(12) {
        let health = format!("{:?}", service.health).to_lowercase();
        lines.push(Line::from(vec![
            Span::styled(format!("#{} ", service.id), theme::accent()),
            Span::styled(
                format!("[{health}] "),
                theme::status_badge(theme::health_color(service.health)),
            ),
            Span::raw(" "),
            Span::styled(service.name.clone(), theme::primary()),
            Span::raw(" "),
            Span::styled(format!("pid:{:?}", service.pid), theme::muted()),
        ]));
    }
    lines
}

fn processes_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Tracked processes", theme::section_title()),
        Span::styled("  active and recently completed jobs", theme::muted()),
    ])];
    if state.processes.snapshots.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No tracked processes.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Foreground and background commands populate this view.",
            theme::muted(),
        )]));
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
        let color = match process.status {
            ProcessStatus::Running => theme::SUCCESS,
            ProcessStatus::Starting => theme::INFO,
            ProcessStatus::Exited => theme::TEXT_SUBTLE,
            ProcessStatus::Failed => theme::ERROR,
            ProcessStatus::Cancelled => theme::WARN,
        };
        lines.push(Line::from(vec![
            Span::styled(format!("[{status}] "), theme::status_badge(color)),
            Span::raw(" "),
            Span::styled(process.label.clone(), theme::primary()),
            Span::raw(" "),
            Span::styled(format!("pid:{:?}", process.pid), theme::muted()),
        ]));
    }
    lines
}

fn log_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Recent logs", theme::section_title()),
        Span::styled("  newest entries first", theme::muted()),
    ])];
    if state.logs.recent.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No logs captured yet.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Run a command to start filling the log buffer.",
            theme::muted(),
        )]));
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
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", format_timestamp(entry.ts)), theme::subtle()),
            Span::styled(
                format!("[{level}] "),
                theme::status_badge(theme::severity_color(entry.severity)),
            ),
            Span::raw(" "),
            Span::styled(entry.raw.clone(), theme::primary()),
        ]));
    }
    lines
}

fn git_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Repository context", theme::section_title()),
        Span::styled("  current branch and changed files", theme::muted()),
    ])];
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("Branch ", theme::muted()),
        Span::styled(
            state
                .git
                .branch
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            theme::primary(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("HEAD   ", theme::muted()),
        Span::styled(
            state
                .git
                .head
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            theme::primary(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Status ", theme::muted()),
        Span::styled(
            if state.git.is_dirty { "dirty" } else { "clean" },
            if state.git.is_dirty {
                theme::accent()
            } else {
                theme::muted()
            },
        ),
    ]));
    lines.push(Line::raw(""));
    if state.git.changed_files.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Changed files ", theme::muted()),
            Span::styled("none", theme::primary()),
        ]));
    } else {
        lines.push(Line::from(vec![Span::styled(
            "Changed files",
            theme::section_title(),
        )]));
        for file in state.git.changed_files.iter().take(10) {
            lines.push(Line::from(vec![
                Span::styled("• ", theme::accent()),
                Span::styled(file.clone(), theme::primary()),
            ]));
        }
    }
    lines
}

fn test_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Test runs", theme::section_title()),
        Span::styled(
            "  normalized test results will accumulate here",
            theme::muted(),
        ),
    ])];
    if state.tests.recent_runs.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No test runs observed yet.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Future adapters should normalize test results here.",
            theme::muted(),
        )]));
        return lines;
    }

    for run in state.tests.recent_runs.iter().rev().take(8) {
        lines.push(Line::from(vec![
            Span::styled(format!("[{:?}] ", run.status), theme::accent()),
            Span::styled(run.runner.clone(), theme::primary()),
            Span::raw(" "),
            Span::styled(
                format!("pass:{} fail:{}", run.pass_count, run.fail_count),
                theme::muted(),
            ),
        ]));
    }
    lines
}

fn render_tabs(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let tabs = DashboardTab::all()
        .iter()
        .map(|tab| {
            let active = *tab == state.ui.dashboard_tab;
            let label = format!(" {} ", tab.title());
            if active {
                Span::styled(label, theme::status_badge(theme::ACCENT))
            } else {
                Span::styled(label, theme::muted())
            }
        })
        .collect::<Vec<_>>();

    let mut content = Vec::new();
    for (index, tab) in tabs.into_iter().enumerate() {
        if index > 0 {
            content.push(Span::raw(" "));
        }
        content.push(tab);
    }
    content.push(Span::raw("  "));
    content.push(Span::styled("Left/Right", theme::accent()));
    content.push(Span::styled(" switch tabs", theme::muted()));

    frame.render_widget(
        Paragraph::new(Line::from(content)).block(Block::default()),
        area,
    );
}

fn render_context(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let summary = match state.ui.dashboard_tab {
        DashboardTab::Services => {
            format!("{} tracked services", state.services.registry.len())
        }
        DashboardTab::Processes => {
            format!("{} process snapshots", state.processes.snapshots.len())
        }
        DashboardTab::Logs => {
            format!("{} buffered log lines", state.logs.recent.len())
        }
        DashboardTab::Git => {
            format!(
                "branch {}  |  {} changed files",
                state
                    .git
                    .branch
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                state.git.changed_files.len()
            )
        }
        DashboardTab::Tests => {
            format!("{} recorded test runs", state.tests.recent_runs.len())
        }
    };

    let line = Line::from(vec![
        Span::styled(state.ui.dashboard_tab.title(), theme::accent()),
        Span::styled("  ", theme::muted()),
        Span::styled(summary, theme::muted()),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
