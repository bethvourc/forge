use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, TimelineKind};
use crate::shared::time::format_timestamp;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let lines = build_timeline_lines(state, area.height.saturating_sub(2) as usize);

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(Span::styled(" Event Stream ", theme::panel_title(focused)))
                    .borders(Borders::ALL)
                    .border_style(theme::pane_border(focused))
                    .style(theme::primary().bg(theme::BG_BASE)),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn build_timeline_lines(state: &AppState, limit: usize) -> Vec<Line<'static>> {
    let mut collapsed: Vec<CollapsedEntry> = Vec::new();

    for entry in state.timeline.entries.iter().rev() {
        if let Some(last) = collapsed.last_mut() {
            if last.kind == entry.kind && last.message == entry.message {
                last.count += 1;
                last.last_seen = entry.at;
                continue;
            }
        }
        collapsed.push(CollapsedEntry {
            at: entry.at,
            last_seen: entry.at,
            kind: entry.kind,
            message: entry.message.clone(),
            count: 1,
        });
        if collapsed.len() >= limit {
            break;
        }
    }

    if collapsed.is_empty() {
        return vec![Line::from(vec![Span::styled(
            "No events yet. Run a command to populate the timeline.",
            theme::muted(),
        )])];
    }

    collapsed
        .into_iter()
        .map(|entry| {
            let kind = match entry.kind {
                TimelineKind::System => "SYSTEM",
                TimelineKind::Ai => "AI",
                TimelineKind::Command => "COMMAND",
                TimelineKind::Log => "LOG",
                TimelineKind::Approval => "APPROVAL",
                TimelineKind::Error => "ERROR",
            };
            let mut spans = vec![
                Span::styled(format!("{} ", format_timestamp(entry.at)), theme::subtle()),
                Span::styled(
                    format!(" {kind} "),
                    theme::status_badge(theme::timeline_color(entry.kind)),
                ),
                Span::raw(" "),
                Span::styled(entry.message, theme::primary()),
            ];
            if entry.count > 1 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(format!("×{}", entry.count), theme::muted()));
            }
            Line::from(spans)
        })
        .collect()
}

struct CollapsedEntry {
    at: std::time::SystemTime,
    last_seen: std::time::SystemTime,
    kind: TimelineKind,
    message: String,
    count: usize,
}
