use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::commands::slash_palette_active;
use crate::domain::{AiStatus, AppState, FocusTarget, ModalState};
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let branch = state
        .git
        .branch
        .clone()
        .unwrap_or_else(|| "no-branch".to_string());
    let head = state
        .git
        .head
        .clone()
        .unwrap_or_else(|| "------".to_string());
    let dirty = if state.git.is_dirty { "dirty" } else { "clean" };

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    frame.render_widget(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::pane_border(false))
            .style(theme::chrome_surface()),
        area,
    );

    let row1 = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(36), Constraint::Length(42)])
        .split(rows[0]);

    frame.render_widget(
        Paragraph::new(primary_identity_line(state, &branch)),
        row1[0],
    );
    frame.render_widget(
        Paragraph::new(status_summary_line(state, dirty, &head))
            .alignment(ratatui::layout::Alignment::Right),
        row1[1],
    );

    frame.render_widget(Paragraph::new(contextual_hint_line(state)), rows[1]);
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

fn truncate_middle(value: &str, max_chars: usize) -> String {
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let side = max_chars.saturating_sub(1) / 2;
    let start = value.chars().take(side).collect::<String>();
    let end = value
        .chars()
        .rev()
        .take(side)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{start}…{end}")
}

fn contextual_hint_line(state: &AppState) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            format!(" {} ", active_surface_label(state)),
            if state.ui.modal.is_some() {
                theme::focus_badge()
            } else {
                match state.ui.focus {
                    FocusTarget::CommandPane => theme::focus_badge(),
                    FocusTarget::DashboardPane | FocusTarget::EventStream => theme::quiet_badge(),
                    FocusTarget::Modal => theme::focus_badge(),
                }
            },
        ),
        Span::raw(" "),
    ];

    let mut hint_spans = match state.ui.modal.as_ref() {
        Some(ModalState::Approval(_)) => vec![
            Span::styled("Enter", theme::keycap()),
            Span::styled(" approve", theme::muted()),
            Span::raw("  "),
            Span::styled("Esc", theme::keycap()),
            Span::styled(" deny", theme::muted()),
            Span::raw("  "),
            Span::styled("/help", theme::accent()),
            Span::styled(" commands", theme::muted()),
        ],
        Some(ModalState::Help) | Some(ModalState::Error(_)) => vec![
            Span::styled("Esc", theme::accent()),
            Span::styled(" close", theme::muted()),
            Span::raw("  "),
            Span::styled("Enter", theme::accent()),
            Span::styled(" dismiss", theme::muted()),
        ],
        None => match state.ui.focus {
            FocusTarget::CommandPane => {
                if slash_palette_active(&state.ui.input.buffer) {
                    vec![
                        Span::styled("↑/↓", theme::keycap()),
                        Span::styled(" choose", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Tab", theme::keycap()),
                        Span::styled(" insert", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Enter", theme::keycap()),
                        Span::styled(" run", theme::muted()),
                    ]
                } else {
                    vec![
                        Span::styled("Enter", theme::keycap()),
                        Span::styled(" run", theme::muted()),
                        Span::raw("  "),
                        Span::styled("/", theme::keycap()),
                        Span::styled(" commands", theme::muted()),
                        Span::raw("  "),
                        Span::styled("F2", theme::keycap()),
                        Span::styled(" mode", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Ctrl+P/N", theme::keycap()),
                        Span::styled(" history", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Ctrl+J", theme::keycap()),
                        Span::styled(" newline", theme::muted()),
                    ]
                }
            }
            FocusTarget::DashboardPane => vec![
                Span::styled("Left/Right", theme::keycap()),
                Span::styled(" switch tabs", theme::muted()),
                Span::raw("  "),
                Span::styled("Tab", theme::keycap()),
                Span::styled(" next pane", theme::muted()),
                Span::raw("  "),
                Span::styled("/", theme::keycap()),
                Span::styled(" commands", theme::muted()),
            ],
            FocusTarget::EventStream => vec![
                Span::styled("Tab", theme::keycap()),
                Span::styled(" next pane", theme::muted()),
                Span::raw("  "),
                Span::styled("/", theme::keycap()),
                Span::styled(" commands", theme::muted()),
            ],
            FocusTarget::Modal => vec![
                Span::styled("Esc", theme::keycap()),
                Span::styled(" close", theme::muted()),
            ],
        },
    };

    spans.append(&mut hint_spans);
    Line::from(spans)
}

fn primary_identity_line(state: &AppState, branch: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(" FORGE ", theme::brand_badge()),
        Span::raw("  "),
        Span::styled(
            truncate_end(&state.project.name, 20),
            theme::primary_emphasis(),
        ),
        Span::styled("  /  ", theme::subtle()),
        Span::styled(truncate_middle(branch, 32), theme::muted()),
    ])
}

fn status_summary_line(state: &AppState, dirty: &str, head: &str) -> Line<'static> {
    let input_mode = state.ui.input.mode.label().to_ascii_lowercase();
    let ai_status = match state.ai.status {
        AiStatus::Disabled => "ai off",
        AiStatus::Unconfigured => "ai setup",
        AiStatus::Ready => "ai ready",
        AiStatus::Queued => "ai queued",
        AiStatus::Running => "ai running",
        AiStatus::Completed => "ai done",
        AiStatus::Failed => "ai error",
    };

    let mut spans = vec![
        Span::styled(
            format!(" {} ", dirty),
            if state.git.is_dirty {
                theme::status_badge(theme::WARN)
            } else {
                theme::quiet_badge()
            },
        ),
        Span::raw(" "),
        Span::styled(format!(" {} ", input_mode), theme::quiet_badge()),
        Span::raw(" "),
        Span::styled(format!(" {} ", ai_status), ai_status_style(state.ai.status)),
    ];

    if !state.approvals.pending.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!(" {} pending ", state.approvals.pending.len()),
            theme::status_badge(theme::WARN),
        ));
    }

    if state.ui.modal.is_some() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(" review ", theme::status_badge(theme::WARN)));
    } else if matches!(
        state.ui.focus,
        FocusTarget::DashboardPane | FocusTarget::EventStream
    ) {
        let focus_label = match state.ui.focus {
            FocusTarget::DashboardPane => "dashboard",
            FocusTarget::EventStream => "activity",
            _ => "",
        };
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!(" {} ", focus_label),
            theme::quiet_badge(),
        ));
    }

    spans.push(Span::raw("  "));
    spans.push(Span::styled("head", theme::label()));
    spans.push(Span::raw(" "));
    spans.push(Span::styled(head.to_string(), theme::subtle()));

    Line::from(spans)
}

fn ai_status_style(status: AiStatus) -> ratatui::style::Style {
    match status {
        AiStatus::Disabled => theme::quiet_badge(),
        AiStatus::Unconfigured => theme::status_badge(theme::WARN),
        AiStatus::Ready | AiStatus::Completed => theme::quiet_badge(),
        AiStatus::Queued | AiStatus::Running => theme::status_badge(theme::INFO),
        AiStatus::Failed => theme::status_badge(theme::ERROR),
    }
}

fn active_surface_label(state: &AppState) -> &'static str {
    match state.ui.modal.as_ref() {
        Some(ModalState::Approval(_)) => "review",
        Some(ModalState::Help) => "help",
        Some(ModalState::Error(_)) => "error",
        None => match state.ui.focus {
            FocusTarget::CommandPane => "command deck",
            FocusTarget::DashboardPane => "operations",
            FocusTarget::EventStream => "activity",
            FocusTarget::Modal => "modal",
        },
    }
}
