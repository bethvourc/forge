use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, DashboardTab, ProcessStatus};
use crate::shared::time::format_timestamp;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    // Outer drawer surface with subtle left hairline.
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(theme::pane_border(focused))
        .style(theme::panel_surface(focused))
        .padding(Padding::new(3, 3, 1, 1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // drawer header
            Constraint::Length(2), // tabs
            Constraint::Length(1), // tab underline / spacer
            Constraint::Min(6),    // content
            Constraint::Length(3), // footer hint
        ])
        .split(inner);

    render_drawer_header(frame, sections[0]);
    render_tabs(frame, sections[1], state);
    render_tabs_separator(frame, sections[2]);

    match state.ui.dashboard_tab {
        DashboardTab::Services => render_services(frame, sections[3], state),
        DashboardTab::Processes => render_lines(frame, sections[3], processes_lines(state)),
        DashboardTab::Logs => render_lines(frame, sections[3], log_lines(state)),
        DashboardTab::Git => render_lines(frame, sections[3], git_lines(state)),
        DashboardTab::Tests => render_lines(frame, sections[3], test_lines(state)),
    }

    render_drawer_footer(frame, sections[4], state);
}

fn render_lines(frame: &mut Frame<'_>, area: Rect, lines: Vec<Line<'static>>) {
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_drawer_header(frame: &mut Frame<'_>, area: Rect) {
    let mid = area.width / 2;
    let left = Rect {
        x: area.x,
        y: area.y,
        width: mid,
        height: 1,
    };
    let right = Rect {
        x: area.x + mid,
        y: area.y,
        width: area.width.saturating_sub(mid),
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "operations",
            theme::italic_serif(),
        )])),
        left,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Esc ", theme::keycap()),
            Span::styled(" close", theme::muted()),
        ]))
        .alignment(ratatui::layout::Alignment::Right),
        right,
    );
}

fn render_tabs_separator(frame: &mut Frame<'_>, area: Rect) {
    // Render a hairline under the tab row.
    let line = "─".repeat(area.width as usize);
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(line, theme::subtle())])),
        area,
    );
}

fn render_services(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if state.services.registry.is_empty() {
        let lines = vec![
            Line::raw(""),
            Line::from(vec![Span::styled(
                "No managed services yet.",
                theme::muted(),
            )]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Launch one with ", theme::subtle()),
                Span::styled("/bg cargo run", theme::accent()),
                Span::styled(" or any shell command that ends with ", theme::subtle()),
                Span::styled("&", theme::accent()),
                Span::styled(".", theme::subtle()),
            ]),
        ];
        render_lines(frame, area, lines);
        return;
    }

    let mut y = area.y;
    for (i, s) in state.services.registry.iter().rev().take(3).enumerate() {
        let card_h: u16 = 7;
        if y + card_h > area.y + area.height {
            break;
        }
        let card = Rect {
            x: area.x,
            y,
            width: area.width,
            height: card_h,
        };
        render_service_card(frame, card, s, state, i);
        y += card_h + 1;
    }
}

fn render_service_card(
    frame: &mut Frame<'_>,
    area: Rect,
    service: &crate::domain::ServiceRecord,
    state: &AppState,
    _index: usize,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::pane_border(false))
        .style(theme::panel_surface(false))
        .padding(Padding::new(2, 2, 0, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let port = service
        .ports
        .first()
        .map(|p| format!(":{p}"))
        .unwrap_or_default();
    let cmd = state
        .commands
        .records
        .iter()
        .find(|c| Some(c.id) == service.linked_command)
        .map(|c| c.raw.clone())
        .unwrap_or_else(|| format!("{:?} service", service.source).to_lowercase());
    let uptime = service
        .last_seen
        .elapsed()
        .map(|d| {
            let secs = d.as_secs();
            if secs < 60 {
                format!("{secs}s")
            } else {
                format!("{}m {}s", secs / 60, secs % 60)
            }
        })
        .unwrap_or_else(|_| "—".to_string());

    let mid = inner.width / 2;
    let header_left = Rect {
        x: inner.x,
        y: inner.y,
        width: mid,
        height: 1,
    };
    let header_right = Rect {
        x: inner.x + mid,
        y: inner.y,
        width: inner.width.saturating_sub(mid),
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("● ", theme::accent()),
            Span::styled(service.name.clone(), theme::primary_emphasis()),
            Span::raw("  "),
            Span::styled(port, theme::accent()),
        ])),
        header_left,
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" logs ", theme::keycap()),
            Span::raw(" "),
            Span::styled(" restart ", theme::keycap()),
            Span::raw(" "),
            Span::styled(" stop ", theme::keycap()),
        ]))
        .alignment(ratatui::layout::Alignment::Right),
        header_right,
    );

    // Command line.
    let cmd_row = Rect {
        x: inner.x,
        y: inner.y + 1,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            truncate(&cmd, 60),
            theme::muted(),
        )])),
        cmd_row,
    );

    // Stats row (PID / UP / CPU / MEM).
    let stats_row = Rect {
        x: inner.x,
        y: inner.y + 2,
        width: inner.width,
        height: 1,
    };
    let pid_str = service
        .pid
        .map(|p| p.to_string())
        .unwrap_or_else(|| "—".into());
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("PID ", theme::subtle()),
            Span::styled(format!("{:<8}", pid_str), theme::muted()),
            Span::styled("UP ", theme::subtle()),
            Span::styled(format!("{:<10}", uptime), theme::muted()),
            Span::styled("CPU ", theme::subtle()),
            Span::styled("—      ", theme::muted()),
            Span::styled("MEM ", theme::subtle()),
            Span::styled("—", theme::muted()),
        ])),
        stats_row,
    );

    // Sparkline row — single line using block chars.
    let spark_row = Rect {
        x: inner.x,
        y: inner.y + 3,
        width: inner.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            sparkline_text(inner.width as usize),
            theme::accent(),
        )])),
        spark_row,
    );
}

fn sparkline_text(width: usize) -> String {
    const CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▄', '▃', '▄', '▅', '▆', '▅', '▄'];
    let mut out = String::new();
    for i in 0..width {
        out.push(CHARS[i % CHARS.len()]);
    }
    out
}

fn render_drawer_footer(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if !matches!(state.ui.dashboard_tab, DashboardTab::Services) {
        return;
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::pane_border(false))
        .style(theme::panel_surface(false));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("launch another with ", theme::subtle()),
            Span::styled("/bg <cmd>", theme::primary()),
            Span::styled(" or suffix ", theme::subtle()),
            Span::styled("&", theme::primary()),
        ]))
        .alignment(ratatui::layout::Alignment::Center),
        inner,
    );
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let kept: String = value.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    let mut spans: Vec<Span<'static>> = Vec::new();
    for tab in DashboardTab::all() {
        let active = tab == state.ui.dashboard_tab;
        let name_style = if active {
            theme::primary_emphasis()
        } else {
            theme::muted()
        };
        spans.push(Span::styled(tab.title().to_ascii_lowercase(), name_style));
        let count = match tab {
            DashboardTab::Services => state.services.registry.len(),
            DashboardTab::Processes => state.processes.snapshots.len(),
            DashboardTab::Logs => state.logs.recent.len(),
            DashboardTab::Git => state.git.changed_files.len(),
            DashboardTab::Tests => state.tests.recent_runs.len(),
        };
        if count > 0 {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!(" {count} "),
                if active {
                    theme::focus_badge()
                } else {
                    theme::quiet_badge()
                },
            ));
        }
        spans.push(Span::raw("    "));
    }

    // Underline marker row below.
    let tab_line = Line::from(spans);

    // Build underline segment roughly beneath active tab name.
    let active_idx = DashboardTab::all()
        .iter()
        .position(|t| *t == state.ui.dashboard_tab)
        .unwrap_or(0);
    let underline = build_tab_underline(state, active_idx, area.width as usize);

    frame.render_widget(Paragraph::new(vec![tab_line, Line::from(underline)]), area);
}

fn build_tab_underline(state: &AppState, active_idx: usize, _width: usize) -> Vec<Span<'static>> {
    let mut acc = String::new();
    for (i, tab) in DashboardTab::all().iter().enumerate() {
        let title_len = tab.title().chars().count();
        let count = match tab {
            DashboardTab::Services => state.services.registry.len(),
            DashboardTab::Processes => state.processes.snapshots.len(),
            DashboardTab::Logs => state.logs.recent.len(),
            DashboardTab::Git => state.git.changed_files.len(),
            DashboardTab::Tests => state.tests.recent_runs.len(),
        };
        let badge_len = if count > 0 {
            count.to_string().chars().count() + 3
        } else {
            0
        };
        let block_len = title_len + badge_len;
        if i == active_idx {
            return vec![
                Span::raw(acc),
                Span::styled("▔".repeat(block_len), theme::accent()),
            ];
        }
        acc.push_str(&" ".repeat(block_len + 4));
    }
    vec![Span::raw(acc)]
}

#[allow(dead_code)]
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
