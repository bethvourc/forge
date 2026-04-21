use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

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

    let project = truncate_end(&state.project.name, 18);

    let running = state
        .services
        .registry
        .iter()
        .filter(|s| {
            matches!(
                s.health,
                crate::domain::ServiceHealth::Healthy
                    | crate::domain::ServiceHealth::Starting
                    | crate::domain::ServiceHealth::Degraded
            )
        })
        .count();

    let left = vec![
        Span::raw("  "),
        Span::styled("forge", theme::italic_serif()),
        Span::styled("  /  ", theme::subtle()),
        Span::styled(project, theme::muted()),
        Span::styled("  /  ", theme::subtle()),
        Span::styled(truncate_middle(&branch, 22), theme::muted()),
    ];

    let mut right: Vec<Span<'static>> = Vec::new();
    if running > 0 {
        right.push(Span::styled("●", theme::accent()));
        right.push(Span::raw(" "));
        right.push(Span::styled(
            format!("{running} running"),
            theme::primary(),
        ));
        right.push(Span::styled("  ·  ", theme::subtle()));
    }
    right.push(Span::styled(
        dirty,
        if state.git.is_dirty {
            theme::warn_accent()
        } else {
            theme::muted()
        },
    ));
    right.push(Span::styled("  ·  ", theme::subtle()));
    right.push(Span::styled(truncate_end(&head, 8), theme::subtle()));
    right.push(Span::raw("  "));

    let mut spans = left;
    spans.extend(right);

    // Two paragraphs: left-aligned + right-aligned — split area in half.
    let mid = area.width / 2;
    let left_area = Rect {
        x: area.x,
        y: area.y,
        width: mid,
        height: 1,
    };
    let right_area = Rect {
        x: area.x + mid,
        y: area.y,
        width: area.width.saturating_sub(mid),
        height: 1,
    };

    // Extract left portion from our combined spans — actually we built them separately
    let left_spans = spans[..6].to_vec();
    let right_spans = spans[6..].to_vec();

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
