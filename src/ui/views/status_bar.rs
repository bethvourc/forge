use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::commands::slash_palette_active;
use crate::domain::AppState;
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
    let mode = if state.ui.modal.is_some() {
        "approval"
    } else if !state.approvals.pending.is_empty() {
        "review"
    } else {
        "normal"
    };
    let focus = match state.ui.focus {
        crate::domain::FocusTarget::CommandPane => "command",
        crate::domain::FocusTarget::DashboardPane => "dashboard",
        crate::domain::FocusTarget::EventStream => "events",
        crate::domain::FocusTarget::Modal => "modal",
    };
    let (ai_label, ai_color) = match state.ai.status {
        crate::domain::AiStatus::Disabled => ("OFF", theme::TEXT_SUBTLE),
        crate::domain::AiStatus::Unconfigured => ("SETUP", theme::WARN),
        crate::domain::AiStatus::Ready => ("READY", theme::SUCCESS),
        crate::domain::AiStatus::Queued => ("QUEUED", theme::INFO),
        crate::domain::AiStatus::Running => ("RUN", theme::INFO),
        crate::domain::AiStatus::Completed => ("DONE", theme::SUCCESS),
        crate::domain::AiStatus::Failed => ("ERROR", theme::ERROR),
    };
    let pending_count = state.approvals.pending.len().to_string();

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    frame.render_widget(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(theme::pane_border(false))
            .style(theme::panel_surface(false)),
        area,
    );

    let row1 = Line::from(vec![
        Span::styled(" FORGE ", theme::status_badge(theme::ACCENT)),
        Span::raw(" "),
        badge(
            "GIT",
            dirty,
            if state.git.is_dirty {
                theme::WARN
            } else {
                theme::SUCCESS
            },
        ),
        Span::raw(" "),
        badge("MODE", mode, theme::INFO),
        Span::raw(" "),
        badge("FOCUS", focus, theme::ACCENT),
        Span::raw(" "),
        badge("AI", ai_label, ai_color),
        Span::raw(" "),
        badge(
            "PENDING",
            &pending_count,
            if state.approvals.pending.is_empty() {
                theme::TEXT_SUBTLE
            } else {
                theme::WARN
            },
        ),
    ]);
    frame.render_widget(Paragraph::new(row1), rows[0]);

    let row2 = Line::from(vec![
        Span::styled("repo ", theme::muted()),
        Span::styled(truncate_end(&state.project.name, 24), theme::primary()),
        Span::raw("  "),
        Span::styled("branch ", theme::muted()),
        Span::styled(truncate_middle(&branch, 32), theme::primary()),
        Span::raw("  "),
        Span::styled("head ", theme::muted()),
        Span::styled(head, theme::primary()),
    ]);
    frame.render_widget(Paragraph::new(row2), rows[1]);

    let row3 = contextual_hint_line(state);
    frame.render_widget(Paragraph::new(row3), rows[2]);
}

fn badge<'a>(label: &'a str, value: &'a str, bg: ratatui::style::Color) -> Span<'a> {
    Span::styled(format!(" {label}:{value} "), theme::status_badge(bg))
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
    match state.ui.modal.as_ref() {
        Some(crate::domain::ModalState::Approval(_)) => Line::from(vec![
            Span::styled("Enter", theme::accent()),
            Span::styled(" approve", theme::muted()),
            Span::raw("  "),
            Span::styled("Esc", theme::accent()),
            Span::styled(" deny", theme::muted()),
            Span::raw("  "),
            Span::styled("/help", theme::accent()),
            Span::styled(" commands", theme::muted()),
        ]),
        Some(crate::domain::ModalState::Help) | Some(crate::domain::ModalState::Error(_)) => {
            Line::from(vec![
                Span::styled("Esc", theme::accent()),
                Span::styled(" close", theme::muted()),
                Span::raw("  "),
                Span::styled("Enter", theme::accent()),
                Span::styled(" dismiss", theme::muted()),
            ])
        }
        None => match state.ui.focus {
            crate::domain::FocusTarget::CommandPane => {
                if slash_palette_active(&state.ui.input_buffer) {
                    Line::from(vec![
                        Span::styled("↑/↓", theme::keycap()),
                        Span::styled(" choose", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Tab", theme::keycap()),
                        Span::styled(" insert", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Enter", theme::keycap()),
                        Span::styled(" run", theme::muted()),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled("/", theme::keycap()),
                        Span::styled(" palette", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Enter", theme::keycap()),
                        Span::styled(" run", theme::muted()),
                        Span::raw("  "),
                        Span::styled("Tab", theme::keycap()),
                        Span::styled(" next pane", theme::muted()),
                        Span::raw("  "),
                        Span::styled("/help", theme::accent()),
                        Span::styled(" commands", theme::muted()),
                    ])
                }
            }
            crate::domain::FocusTarget::DashboardPane => Line::from(vec![
                Span::styled("Left/Right", theme::keycap()),
                Span::styled(" switch tabs", theme::muted()),
                Span::raw("  "),
                Span::styled("Tab", theme::keycap()),
                Span::styled(" next pane", theme::muted()),
                Span::raw("  "),
                Span::styled("F1", theme::keycap()),
                Span::styled(" help", theme::muted()),
            ]),
            crate::domain::FocusTarget::EventStream => Line::from(vec![
                Span::styled("Tab", theme::keycap()),
                Span::styled(" next pane", theme::muted()),
                Span::raw("  "),
                Span::styled("F1", theme::keycap()),
                Span::styled(" help", theme::muted()),
            ]),
            crate::domain::FocusTarget::Modal => Line::from(vec![
                Span::styled("Esc", theme::keycap()),
                Span::styled(" close", theme::muted()),
            ]),
        },
    }
}
