use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, ApprovalMode, NotificationLevel};
use crate::shared::time::format_timestamp;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Input",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(format!("> {}", state.ui.input_buffer)));
    lines.push(Line::raw(""));

    if let Some(approval) = state
        .approvals
        .pending
        .iter()
        .find(|request| matches!(request.mode, ApprovalMode::InlineReview))
    {
        lines.push(Line::from(Span::styled(
            "Pending Review",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!(
            "#{} [{}] {}",
            approval.command_id,
            approval.class.label(),
            approval.summary
        )));
        lines.push(Line::from("Press Enter or /approve to run. Esc or /deny to dismiss."));
        lines.push(Line::raw(""));
    }

    lines.push(Line::from(Span::styled(
        "Recent Commands",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    for record in state
        .commands
        .records
        .iter()
        .rev()
        .take(state.config.ui.history_preview)
    {
        lines.push(Line::from(format!(
            "#{} [{:?}] {}",
            record.id, record.status, record.raw
        )));
    }

    if let Some(notification) = state.notifications.items.last() {
        lines.push(Line::raw(""));
        let color = match notification.level {
            NotificationLevel::Info => Color::Blue,
            NotificationLevel::Warn => Color::Yellow,
            NotificationLevel::Error => Color::Red,
        };
        lines.push(Line::from(Span::styled(
            "Notification",
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(format!(
            "[{}] {}",
            format_timestamp(notification.created_at),
            notification.message
        )));
    }

    let block = Block::default()
        .title("Command / AI")
        .borders(Borders::ALL)
        .border_style(border_style);
    let widget = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

