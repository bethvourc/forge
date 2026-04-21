use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::commands::{slash_command_suggestions, SlashCommandSpec};
use crate::domain::{AiStatus, AppState, ApprovalMode, InputMode};
use crate::ui::theme;

// Hero prompt — Prompt-First editorial layout.
// Narrow content column centered in the available area.
pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    // Center a content column roughly 90 cells wide (clamped to area).
    let column_w = area.width.min(96).max(40);
    let pad = area.width.saturating_sub(column_w) / 2;
    let column = Rect {
        x: area.x + pad,
        y: area.y + 1,
        width: column_w,
        height: area.height.saturating_sub(1),
    };

    let dim = !focused;
    let slash_suggestions = slash_command_suggestions(&state.ui.input.buffer);
    let visible_suggestions = visible_command_suggestions(&slash_suggestions, state);
    let running_items = running_items(state);
    let running_h: u16 = if running_items.is_empty() {
        0
    } else {
        (running_items.len() as u16) + 3
    };
    let suggestion_h: u16 = if visible_suggestions.is_empty() {
        0
    } else {
        (visible_suggestions.len() as u16) + 2
    };
    let context_h: u16 = if should_show_context_card(state) && suggestion_h == 0 {
        10
    } else {
        0
    };

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(if running_h == 0 { 2 } else { 0 }), // spacer
            Constraint::Length(running_h),
            Constraint::Length(2), // headline
            Constraint::Length(3), // prompt
            Constraint::Length(suggestion_h),
            Constraint::Length(context_h),
            Constraint::Min(0),
        ])
        .split(column);

    if running_h > 0 {
        render_running_strip(frame, sections[1], &running_items);
    }

    render_headline(frame, sections[2], state, dim);
    render_prompt(frame, sections[3], state, dim);

    if suggestion_h > 0 {
        render_suggestions(frame, sections[4], &visible_suggestions);
    } else if context_h > 0 {
        render_context_card(frame, sections[5], state);
    } else {
        render_recent_chips(frame, sections[6], state);
    }
}

fn render_headline(frame: &mut Frame<'_>, area: Rect, state: &AppState, dim: bool) {
    if state.ui.input.buffer.is_empty() && area.height >= 1 {
        let text = match state.ui.input.mode {
            InputMode::Shell => "what do you want to do?",
            InputMode::AiAssist => "what should Forge help you think through?",
            InputMode::AiDiagnose => "what's wrong — describe the symptom.",
        };
        let style = if dim {
            theme::subtle()
        } else {
            theme::italic_muted()
        };
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(text, style)])),
            area,
        );
    }
}

fn render_prompt(frame: &mut Frame<'_>, area: Rect, state: &AppState, dim: bool) {
    let buf = &state.ui.input.buffer;
    let cursor = state.ui.input.cursor;
    let mode = state.ui.input.mode;
    let slash = buf.trim_start().starts_with('/');

    let arrow_color = if dim {
        theme::subtle()
    } else {
        theme::accent()
    };
    let arrow_char = if slash {
        "/"
    } else {
        match mode {
            InputMode::Shell => "▸",
            InputMode::AiAssist | InputMode::AiDiagnose => "✦",
        }
    };

    // Render a 3-line bordered box with the prompt.
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if dim {
            theme::pane_border(false)
        } else {
            theme::pane_border(true)
        })
        .style(theme::panel_surface(!dim))
        .padding(Padding::horizontal(2));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let text_width = inner.width.saturating_sub(4) as usize;
    let display = render_prompt_text(buf, cursor, text_width);

    let spans = if buf.is_empty() {
        let placeholder = match mode {
            InputMode::Shell => "run, ask, commit, deploy…",
            InputMode::AiAssist => "ask Forge anything about this repo…",
            InputMode::AiDiagnose => "describe the symptom to diagnose…",
        };
        vec![
            Span::styled(format!("{arrow_char}  "), arrow_color),
            Span::styled(placeholder, theme::subtle()),
            Span::styled("█", if dim { theme::subtle() } else { theme::primary() }),
        ]
    } else {
        vec![
            Span::styled(format!("{arrow_char}  "), arrow_color),
            Span::styled(
                display,
                if dim {
                    theme::muted()
                } else {
                    theme::primary_emphasis()
                },
            ),
        ]
    };

    frame.render_widget(
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_prompt_text(buffer: &str, cursor: usize, width: usize) -> String {
    let single = buffer.replace('\n', " ⏎ ");
    let chars: Vec<char> = single.chars().collect();
    if chars.len() <= width.max(1) {
        let mut s: String = chars.iter().collect();
        if cursor <= buffer.len() {
            // append block cursor at end for simplicity
            s.push('█');
        }
        return s;
    }
    let start = chars.len().saturating_sub(width.saturating_sub(2));
    let tail: String = chars[start..].iter().collect();
    format!("…{tail}█")
}

fn render_running_strip(frame: &mut Frame<'_>, area: Rect, items: &[RunItem]) {
    let mut lines = vec![
        Line::from(vec![Span::styled(
            "  RUNNING",
            theme::subtle(),
        )]),
        Line::raw(""),
    ];
    for it in items {
        lines.push(Line::from(vec![
            Span::styled("  ● ", theme::accent()),
            Span::styled(
                format!("{:<8}", it.name),
                theme::primary_emphasis(),
            ),
            Span::styled(format!("  {}", it.cmd), theme::muted()),
            Span::styled(format!("   {}", it.meta), theme::subtle()),
        ]));
    }
    lines.push(Line::raw(""));
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        area,
    );
}

struct RunItem {
    name: String,
    cmd: String,
    meta: String,
}

fn running_items(state: &AppState) -> Vec<RunItem> {
    state
        .services
        .registry
        .iter()
        .filter(|s| {
            matches!(
                s.health,
                crate::domain::ServiceHealth::Healthy
                    | crate::domain::ServiceHealth::Starting
            )
        })
        .take(3)
        .map(|s| RunItem {
            name: s.name.clone(),
            cmd: if s.ports.is_empty() {
                format!("{:?}", s.source).to_lowercase()
            } else {
                format!(
                    "{} · :{}",
                    format!("{:?}", s.source).to_lowercase(),
                    s.ports
                        .iter()
                        .map(u16::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            meta: format!("pid {}", s.pid.map(|p| p.to_string()).unwrap_or_default()),
        })
        .collect()
}

fn render_suggestions(
    frame: &mut Frame<'_>,
    area: Rect,
    suggestions: &[(bool, &SlashCommandSpec)],
) {
    let mut lines = vec![Line::from(vec![Span::styled(
        "  SUGGESTIONS",
        theme::subtle(),
    )])];
    for (selected, spec) in suggestions {
        let mode_pill = Span::styled(
            " slash ",
            if *selected {
                theme::focus_badge()
            } else {
                theme::quiet_badge()
            },
        );
        let cmd_style = if *selected {
            theme::primary_emphasis()
        } else {
            theme::muted()
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            mode_pill,
            Span::raw("  "),
            Span::styled(format!("{:<22}", spec.usage), cmd_style),
            Span::styled(format!("  {}", spec.summary), theme::subtle()),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }),
        area,
    );
}

fn should_show_context_card(state: &AppState) -> bool {
    if state
        .approvals
        .pending
        .iter()
        .any(|r| matches!(r.mode, ApprovalMode::InlineReview))
    {
        return true;
    }
    if state.ui.input.mode.ai_kind().is_some() {
        return true;
    }
    if matches!(
        state.ai.status,
        AiStatus::Running | AiStatus::Queued | AiStatus::Failed | AiStatus::Completed
    ) {
        return true;
    }
    state.commands.records.last().is_some()
}

fn render_context_card(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let (title, body) = build_context_card(state);
    let block = Block::default()
        .borders(Borders::LEFT)
        .border_style(theme::pane_border(true))
        .style(theme::panel_surface(false))
        .padding(Padding::new(2, 2, 1, 1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(vec![Span::styled(title, theme::subtle())])];
    lines.push(Line::raw(""));
    lines.extend(body);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inner,
    );
}

fn build_context_card(state: &AppState) -> (String, Vec<Line<'static>>) {
    if let Some(approval) = state
        .approvals
        .pending
        .iter()
        .find(|r| matches!(r.mode, ApprovalMode::InlineReview))
    {
        let title = format!("REVIEW · #{}", approval.command_id);
        let body = vec![
            Line::from(vec![Span::styled(
                approval.summary.clone(),
                theme::primary_emphasis(),
            )]),
            Line::raw(""),
            Line::from(vec![Span::styled(
                truncate(&approval.detail, 280),
                theme::muted(),
            )]),
            Line::raw(""),
            Line::from(vec![
                Span::styled(" Enter ", theme::keycap()),
                Span::styled(" approve    ", theme::muted()),
                Span::styled(" Esc ", theme::keycap()),
                Span::styled(" deny", theme::muted()),
            ]),
        ];
        return (title, body);
    }

    if state.ui.input.mode.ai_kind().is_some()
        || matches!(
            state.ai.status,
            AiStatus::Running | AiStatus::Queued | AiStatus::Failed | AiStatus::Completed
        )
    {
        let provider = state
            .ai
            .provider
            .clone()
            .unwrap_or_else(|| "unconfigured".to_string());
        let model = state
            .ai
            .model
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let label = match state.ai.status {
            AiStatus::Disabled => "AI · off",
            AiStatus::Unconfigured => "AI · setup required",
            AiStatus::Ready => "AI · ready",
            AiStatus::Queued => "AI · queued",
            AiStatus::Running => "AI · streaming…",
            AiStatus::Completed => "AI · complete",
            AiStatus::Failed => "AI · error",
        };
        let mut body = vec![Line::from(vec![
            Span::styled(format!("{provider}  "), theme::muted()),
            Span::styled(format!("model {model}"), theme::subtle()),
        ])];
        if let Some(err) = state.ai.last_error.as_ref() {
            body.push(Line::raw(""));
            body.push(Line::from(vec![Span::styled(
                truncate(err, 240),
                theme::error_accent(),
            )]));
        } else if let Some(resp) = state.ai.last_response.as_ref() {
            body.push(Line::raw(""));
            body.push(Line::from(vec![Span::styled(
                truncate(&resp.summary, 240),
                theme::italic_serif(),
            )]));
            if !resp.recommendations.is_empty() || !resp.proposals.is_empty() {
                body.push(Line::raw(""));
                body.push(Line::from(vec![Span::styled(
                    format!(
                        "{} recommendation(s) · {} proposal(s)",
                        resp.recommendations.len(),
                        resp.proposals.len()
                    ),
                    theme::subtle(),
                )]));
            }
        } else if let Some(req) = state.ai.requests.last() {
            body.push(Line::raw(""));
            body.push(Line::from(vec![Span::styled(
                truncate(&req.prompt, 240),
                theme::muted(),
            )]));
        }
        return (label.to_string(), body);
    }

    if let Some(cmd) = state.commands.records.last() {
        let title = format!(
            "LAST RUN · #{}  {}",
            cmd.id,
            status_label(cmd.status)
        );
        let body = vec![
            Line::from(vec![Span::styled(
                truncate(&cmd.raw, 200),
                theme::primary_emphasis(),
            )]),
            Line::raw(""),
            Line::from(vec![Span::styled(
                format!(
                    "{}  ·  {}",
                    cmd.safety_class.label(),
                    cmd.cwd.display()
                ),
                theme::subtle(),
            )]),
        ];
        return (title, body);
    }

    (
        "READY".to_string(),
        vec![Line::from(vec![Span::styled(
            "Shell is quiet. Press / for commands or F2 to switch modes.",
            theme::muted(),
        )])],
    )
}

fn render_recent_chips(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if area.height == 0 {
        return;
    }
    let recents: Vec<String> = state
        .commands
        .records
        .iter()
        .rev()
        .take(3)
        .map(|c| truncate(&c.raw, 24))
        .collect();
    if recents.is_empty() {
        return;
    }
    let mut spans: Vec<Span<'static>> = vec![Span::styled("  recent  ", theme::subtle())];
    for r in recents {
        spans.push(Span::styled(format!(" {r} "), theme::quiet_badge()));
        spans.push(Span::raw("  "));
    }
    frame.render_widget(
        Paragraph::new(Line::from(spans)).wrap(Wrap { trim: true }),
        area,
    );
}

fn status_label(status: crate::domain::CommandStatus) -> &'static str {
    use crate::domain::CommandStatus::*;
    match status {
        PendingReview => "review",
        PendingApproval => "pending",
        Queued => "queued",
        Running => "running",
        Succeeded => "done",
        Failed => "failed",
        Cancelled => "cancelled",
        Denied => "denied",
    }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let joined = value.lines().collect::<Vec<_>>().join(" ");
    if joined.chars().count() <= max_chars {
        return joined;
    }
    let kept: String = joined.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{kept}…")
}

fn visible_command_suggestions<'a>(
    slash_suggestions: &'a [&'a SlashCommandSpec],
    state: &AppState,
) -> Vec<(bool, &'a SlashCommandSpec)> {
    if slash_suggestions.is_empty() {
        return Vec::new();
    }
    let selection = state
        .ui
        .command_palette_cursor
        .min(slash_suggestions.len() - 1);
    let visible_len = slash_suggestions.len().min(5);
    let max_start = slash_suggestions.len().saturating_sub(visible_len);
    let visible_start = selection
        .saturating_sub(visible_len.saturating_sub(1))
        .min(max_start);
    slash_suggestions
        .iter()
        .skip(visible_start)
        .take(visible_len)
        .enumerate()
        .map(|(offset, suggestion)| (visible_start + offset == selection, *suggestion))
        .collect()
}
