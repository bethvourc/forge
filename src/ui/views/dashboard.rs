use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, DashboardTab, ProcessStatus};
use crate::shared::time::format_timestamp;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let block = Block::default()
        .title(Span::styled(" Operations ", theme::panel_title(focused)))
        .borders(Borders::ALL)
        .border_style(theme::pane_border(focused))
        .style(theme::panel_surface(focused))
        .padding(Padding::horizontal(1));
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

    render_overview(frame, sections[0], state);
    render_tabs(frame, sections[1], state);

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
        Span::styled(
            "  long-lived jobs with health and PID visibility",
            theme::muted(),
        ),
    ])];
    if state.services.registry.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No managed services yet.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![
            Span::styled("Launch one with ", theme::muted()),
            Span::styled("/bg cargo run", theme::accent()),
            Span::styled(" or any shell command that ends with ", theme::muted()),
            Span::styled("&", theme::accent()),
            Span::styled(".", theme::muted()),
        ]));
        return lines;
    }

    for service in state.services.registry.iter().rev().take(12) {
        let health = format!("{:?}", service.health).to_lowercase();
        lines.push(Line::from(vec![
            Span::styled(format!("#{: <3}", service.id), theme::accent()),
            Span::styled(
                format!(" {health} "),
                theme::status_badge(theme::health_color(service.health)),
            ),
            Span::raw(" "),
            Span::styled(truncate_end(&service.name, 28), theme::primary()),
            Span::raw(" "),
            Span::styled(format!("pid:{:?}", service.pid), theme::muted()),
        ]));
    }
    lines
}

fn processes_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Tracked processes", theme::section_title()),
        Span::styled(
            "  foreground work, background jobs, and recent exits",
            theme::muted(),
        ),
    ])];
    if state.processes.snapshots.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No tracked processes.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Run commands in the deck and their lifecycle will accumulate here.",
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
            Span::styled(format!(" {status} "), theme::status_badge(color)),
            Span::raw(" "),
            Span::styled(truncate_end(&process.label, 34), theme::primary()),
            Span::raw(" "),
            Span::styled(format!("pid:{:?}", process.pid), theme::muted()),
        ]));
    }
    lines
}

fn log_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Recent logs", theme::section_title()),
        Span::styled(
            "  most recent entries first across tracked work",
            theme::muted(),
        ),
    ])];
    if state.logs.recent.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![Span::styled(
            "No logs captured yet.",
            theme::primary(),
        )]));
        lines.push(Line::from(vec![Span::styled(
            "Logs appear automatically once commands or services start producing output.",
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
                format!(" {level} "),
                theme::status_badge(theme::severity_color(entry.severity)),
            ),
            Span::raw(" "),
            Span::styled(truncate_end(&entry.raw, 56), theme::primary()),
        ]));
    }
    lines
}

fn git_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Repository context", theme::section_title()),
        Span::styled("  branch, head, and working tree state", theme::muted()),
    ])];
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled("Branch", theme::label()),
        Span::raw(" "),
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
        Span::styled("Head", theme::label()),
        Span::raw("   "),
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
        Span::styled("Status", theme::label()),
        Span::raw(" "),
        Span::styled(
            format!(" {} ", if state.git.is_dirty { "dirty" } else { "clean" }),
            if state.git.is_dirty {
                theme::status_badge(theme::WARN)
            } else {
                theme::quiet_badge()
            },
        ),
    ]));
    lines.push(Line::raw(""));
    if state.git.changed_files.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Changed files", theme::label()),
            Span::raw(" "),
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
                Span::styled(truncate_end(file, 56), theme::primary()),
            ]));
        }
    }
    lines
}

fn test_lines(state: &AppState) -> Vec<Line<'static>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Test runs", theme::section_title()),
        Span::styled(
            "  normalized pass/fail summaries across runners",
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
            "Wire test adapters into Forge and their results will surface here.",
            theme::muted(),
        )]));
        return lines;
    }

    for run in state.tests.recent_runs.iter().rev().take(8) {
        let status = format!("{:?}", run.status).to_ascii_lowercase();
        lines.push(Line::from(vec![
            Span::styled(format!(" {status} "), theme::quiet_badge()),
            Span::raw(" "),
            Span::styled(truncate_end(&run.runner, 24), theme::primary()),
            Span::raw(" "),
            Span::styled(
                format!("pass:{} fail:{}", run.pass_count, run.fail_count),
                theme::muted(),
            ),
        ]));
    }
    lines
}

fn render_overview(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let branch = state
        .git
        .branch
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let lines = vec![
        Line::from(vec![
            Span::styled("services", theme::label()),
            Span::raw(" "),
            Span::styled(state.services.registry.len().to_string(), theme::primary()),
            Span::styled("  •  ", theme::subtle()),
            Span::styled("processes", theme::label()),
            Span::raw(" "),
            Span::styled(
                state.processes.snapshots.len().to_string(),
                theme::primary(),
            ),
            Span::styled("  •  ", theme::subtle()),
            Span::styled("logs", theme::label()),
            Span::raw(" "),
            Span::styled(state.logs.recent.len().to_string(), theme::primary()),
            Span::styled("  •  ", theme::subtle()),
            Span::styled("approvals", theme::label()),
            Span::raw(" "),
            Span::styled(state.approvals.pending.len().to_string(), theme::primary()),
        ]),
        Line::from(vec![
            Span::styled("workspace", theme::quiet_badge()),
            Span::raw(" "),
            Span::styled(truncate_end(&branch, 24), theme::primary()),
            Span::styled("  •  ", theme::subtle()),
            Span::styled(
                if state.git.is_dirty {
                    "dirty tree"
                } else {
                    "clean tree"
                },
                if state.git.is_dirty {
                    theme::warn_accent()
                } else {
                    theme::muted()
                },
            ),
            Span::styled("  •  ", theme::subtle()),
            Span::styled(
                format!("{} changed file(s)", state.git.changed_files.len()),
                theme::muted(),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), area);
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
                Span::styled(label, theme::quiet_badge())
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
    content.push(Span::styled("Left/Right", theme::keycap()));
    content.push(Span::styled(" switch tabs", theme::muted()));

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(content),
            Line::from(vec![Span::styled(
                active_tab_summary(state.ui.dashboard_tab),
                theme::subtle(),
            )]),
        ]),
        area,
    );
}

fn active_tab_summary(tab: DashboardTab) -> &'static str {
    match tab {
        DashboardTab::Services => "Long-lived services with health, PID, and launch visibility.",
        DashboardTab::Processes => "Recent foreground and background command execution history.",
        DashboardTab::Logs => "Unified log flow from commands and tracked services.",
        DashboardTab::Git => "Working tree status, branch context, and changed files.",
        DashboardTab::Tests => "Normalized runner outcomes as adapters come online.",
    }
}

fn truncate_end(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let kept = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{kept}…")
}
