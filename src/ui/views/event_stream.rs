use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, TimelineKind};
use crate::shared::time::format_timestamp;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let lines = state
        .timeline
        .entries
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|entry| {
            let kind = match entry.kind {
                TimelineKind::System => "system",
                TimelineKind::Command => "command",
                TimelineKind::Log => "log",
                TimelineKind::Approval => "approval",
                TimelineKind::Error => "error",
            };
            format!("[{}] {kind}: {}", format_timestamp(entry.at), entry.message)
        })
        .collect::<Vec<_>>();

    frame.render_widget(
        Paragraph::new(lines.join("\n"))
            .block(
                Block::default()
                    .title("Event Stream")
                    .borders(Borders::ALL)
                    .border_style(border_style),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

