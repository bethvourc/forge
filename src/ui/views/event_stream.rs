use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::domain::AppState;
use crate::ui::{keys, theme};

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, _focused: bool) {
    frame.render_widget(Block::default().style(theme::chrome_surface()), area);

    let latest = state
        .timeline
        .entries
        .iter()
        .rev()
        .next()
        .map(|e| truncate_end(&e.message, 60))
        .unwrap_or_else(|| "quiet — waiting on you".to_string());
    let count = state.timeline.entries.len();

    let left_spans = vec![
        Span::raw("  "),
        Span::styled("▸ ", theme::subtle()),
        Span::styled(latest, theme::muted()),
        Span::styled(format!("  ·  {count}"), theme::subtle()),
    ];

    let right_spans = vec![
        Span::styled(format!(" {} ", keys::meta("O")), theme::keycap()),
        Span::styled(" ops", theme::muted()),
        Span::raw("   "),
        Span::styled(format!(" {} ", keys::meta("L")), theme::keycap()),
        Span::styled(" activity", theme::muted()),
        Span::raw("   "),
        Span::styled(format!(" {} ", keys::meta("/")), theme::keycap()),
        Span::styled(" ai", theme::muted()),
        Span::raw("   "),
        Span::styled(format!(" {} ", keys::meta("K")), theme::keycap()),
        Span::styled(" commands", theme::muted()),
        Span::raw("  "),
    ];

    let content_y = if area.height >= 2 { area.y + 1 } else { area.y };
    if area.height >= 2 {
        let separator = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "─".repeat(area.width as usize),
                theme::separator(),
            ))),
            separator,
        );
    }

    let mid = area.width / 2;
    let left_area = Rect {
        x: area.x,
        y: content_y,
        width: mid,
        height: 1,
    };
    let right_area = Rect {
        x: area.x + mid,
        y: content_y,
        width: area.width.saturating_sub(mid),
        height: 1,
    };

    frame.render_widget(
        Paragraph::new(Line::from(left_spans)).style(theme::chrome_surface()),
        left_area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(right_spans))
            .alignment(ratatui::layout::Alignment::Right)
            .style(theme::chrome_surface()),
        right_area,
    );
}

fn truncate_end(value: &str, max_chars: usize) -> String {
    let joined = value.lines().collect::<Vec<_>>().join(" ");
    if joined.chars().count() <= max_chars {
        return joined;
    }
    let kept = joined
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{kept}…")
}
