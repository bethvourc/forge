use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use crate::domain::AppState;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    frame.render_widget(Block::default().style(theme::chrome_surface()), area);

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

    let branch_display = truncate_middle(&branch, 32);
    let left = if project.eq_ignore_ascii_case("forge") {
        vec![
            Span::raw("  "),
            Span::styled("forge", theme::italic_serif()),
            Span::styled("   │   ", theme::subtle()),
            Span::styled(branch_display, theme::status_context()),
        ]
    } else {
        vec![
            Span::raw("  "),
            Span::styled("forge", theme::italic_serif()),
            Span::styled("   │   ", theme::subtle()),
            Span::styled(project, theme::status_context()),
            Span::styled("   │   ", theme::subtle()),
            Span::styled(branch_display, theme::status_context()),
        ]
    };

    let mut right: Vec<Span<'static>> = Vec::new();
    if running > 0 {
        right.push(Span::styled("●", theme::accent()));
        right.push(Span::raw(" "));
        right.push(Span::styled(format!("{running} running"), theme::primary()));
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

    let text_y = if area.height >= 3 { area.y + 1 } else { area.y };
    let text_area = Rect {
        x: area.x,
        y: text_y,
        width: area.width,
        height: 1,
    };

    // Two paragraphs: left-aligned + right-aligned, matching the sparse wireframe bar.
    let mid = text_area.width / 2;
    let left_area = Rect {
        x: text_area.x,
        y: text_area.y,
        width: mid,
        height: 1,
    };
    let right_area = Rect {
        x: text_area.x + mid,
        y: text_area.y,
        width: text_area.width.saturating_sub(mid),
        height: 1,
    };

    frame.render_widget(
        Paragraph::new(Line::from(left)).style(theme::chrome_surface()),
        left_area,
    );
    frame.render_widget(
        Paragraph::new(Line::from(right))
            .alignment(ratatui::layout::Alignment::Right)
            .style(theme::chrome_surface()),
        right_area,
    );

    if area.height > 1 {
        let separator = Rect {
            x: area.x,
            y: area.y + area.height - 1,
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
