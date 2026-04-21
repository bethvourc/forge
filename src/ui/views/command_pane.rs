use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::commands::{slash_command_suggestions, SlashCommandSpec};
use crate::domain::{AiStatus, AppState, ApprovalMode, InputMode};
use crate::shared::time::format_timestamp;
use crate::ui::{keys, theme};

// Intent suggestions shown while typing a non-slash command.
#[derive(Clone)]
struct IntentSuggestion {
    mode: &'static str, // SHELL / BG / AI / HIST
    cmd: String,
    hint: String,
}

// Hero prompt — Prompt-First editorial layout.
// Narrow content column centered in the available area.
pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    // Center a narrow content column (design: ~760px, i.e. ~60% of viewport).
    let column_w = area.width.saturating_mul(60) / 100;
    let column_w = column_w.clamp(48, 92);
    let x_pad = area.width.saturating_sub(column_w) / 2;

    // Vertically center the hero group (headline + prompt + chips).
    let running_items = running_items(state);
    let slash_suggestions = slash_command_suggestions(&state.ui.input.buffer);
    let visible_slash = visible_command_suggestions(&slash_suggestions, state);
    let intent_suggestions = intent_suggestions(state);

    let headline_h: u16 = if state.ui.input.buffer.is_empty() { 2 } else { 0 };
    let prompt_h: u16 = 5;
    let chips_h: u16 = if visible_slash.is_empty()
        && intent_suggestions.is_empty()
        && !should_show_context_card(state)
    {
        2
    } else {
        0
    };
    let running_h: u16 = if running_items.is_empty() {
        0
    } else {
        (running_items.len() as u16) + 3
    };
    let slash_h: u16 = if visible_slash.is_empty() {
        0
    } else {
        (visible_slash.len() as u16) + 2
    };
    let intent_h: u16 = if intent_suggestions.is_empty() {
        0
    } else {
        (intent_suggestions.len() as u16) + 2
    };
    let suggestion_h = slash_h + intent_h;
    let context_h: u16 = if should_show_context_card(state) && suggestion_h == 0 {
        14
    } else {
        0
    };

    let group_h = running_h + headline_h + prompt_h + suggestion_h + context_h + chips_h;
    let y_pad = area.height.saturating_sub(group_h) * 42 / 100;

    let column = Rect {
        x: area.x + x_pad,
        y: area.y + y_pad,
        width: column_w,
        height: area.height.saturating_sub(y_pad),
    };

    let dim = !focused;

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(running_h),
            Constraint::Length(headline_h),
            Constraint::Length(prompt_h),
            Constraint::Length(suggestion_h),
            Constraint::Length(context_h),
            Constraint::Length(chips_h),
            Constraint::Min(0),
        ])
        .split(column);

    if running_h > 0 {
        render_running_strip(frame, sections[0], &running_items);
    }
    if headline_h > 0 {
        render_headline(frame, sections[1], state, dim);
    }
    render_prompt(frame, sections[2], state, dim);

    if suggestion_h > 0 {
        if slash_h > 0 {
            render_slash_suggestions(frame, sections[3], &visible_slash);
        } else {
            render_intent_suggestions(frame, sections[3], &intent_suggestions);
        }
    } else if context_h > 0 {
        render_context_card(frame, sections[4], state);
    } else if chips_h > 0 {
        render_recent_chips(frame, sections[5], state);
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
        .padding(Padding::new(3, 3, 1, 1));
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
    // Heading row.
    let heading = Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled("RUNNING", theme::subtle())])),
        heading,
    );

    // One bordered row per running item.
    let mut y = area.y + 2;
    for it in items {
        if y >= area.y + area.height {
            break;
        }
        let row = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::pane_border(false))
            .style(theme::panel_surface(false));
        // Draw a 3-cell tall framed row by overlaying inner text.
        let framed = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };
        let _ = (block, framed);
        let meta = format!(
            "pid {} · {} · {}",
            it.pid.as_deref().unwrap_or("—"),
            it.uptime,
            it.port.as_deref().unwrap_or("—"),
        );
        let spans = vec![
            Span::styled("● ", theme::accent()),
            Span::styled(format!("{:<8}", it.name), theme::primary_emphasis()),
            Span::styled(format!(" {}", truncate(&it.cmd, 40)), theme::muted()),
            Span::styled(format!("   {}", meta), theme::subtle()),
            Span::styled("   logs ↗", theme::subtle()),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), row);
        y += 1;
    }
}

struct RunItem {
    name: String,
    cmd: String,
    pid: Option<String>,
    uptime: String,
    port: Option<String>,
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
        .map(|s| {
            let uptime = s
                .last_seen
                .elapsed()
                .map(|d| {
                    let secs = d.as_secs();
                    if secs < 60 {
                        format!("{secs}s")
                    } else if secs < 3600 {
                        format!("{}m {}s", secs / 60, secs % 60)
                    } else {
                        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
                    }
                })
                .unwrap_or_else(|_| "—".to_string());
            let port = s.ports.first().map(|p| format!(":{p}"));
            let cmd = state
                .commands
                .records
                .iter()
                .find(|c| Some(c.id) == s.linked_command)
                .map(|c| c.raw.clone())
                .unwrap_or_else(|| format!("{:?} service", s.source).to_lowercase());
            RunItem {
                name: s.name.clone(),
                cmd,
                pid: s.pid.map(|p| p.to_string()),
                uptime,
                port,
            }
        })
        .collect()
}

fn render_slash_suggestions(
    frame: &mut Frame<'_>,
    area: Rect,
    suggestions: &[(bool, &SlashCommandSpec)],
) {
    let mut lines = vec![
        Line::from(vec![Span::styled("SUGGESTIONS", theme::subtle())]),
        Line::raw(""),
    ];
    let width = area.width as usize;
    for (selected, spec) in suggestions {
        let pill = Span::styled(
            " SLASH ",
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
        let hint = truncate(spec.summary, 28);
        let cmd_text = format!("{:<22}", truncate(spec.usage, 22));
        let used = 1 + 7 + 2 + 22 + 2 + hint.chars().count();
        let spacer = " ".repeat(width.saturating_sub(used).saturating_sub(6));
        let mut spans = vec![
            Span::raw(" "),
            pill,
            Span::raw("  "),
            Span::styled(cmd_text, cmd_style),
            Span::raw("  "),
            Span::styled(hint, theme::subtle()),
            Span::raw(spacer),
        ];
        if *selected {
            spans.push(Span::styled(format!(" {} ", keys::enter()), theme::keycap()));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_intent_suggestions(frame: &mut Frame<'_>, area: Rect, items: &[IntentSuggestion]) {
    let mut lines = vec![
        Line::from(vec![Span::styled("SUGGESTIONS", theme::subtle())]),
        Line::raw(""),
    ];
    let width = area.width as usize;
    for (i, it) in items.iter().enumerate() {
        let selected = i == 0;
        let pill_style = if selected {
            theme::focus_badge()
        } else {
            theme::quiet_badge()
        };
        let pill_text = format!(" {:<5}", it.mode);
        let cmd_style = if selected {
            theme::primary_emphasis()
        } else {
            theme::muted()
        };
        let cmd_text = truncate(&it.cmd, 32);
        let hint = truncate(&it.hint, 30);
        let used = 1 + pill_text.chars().count() + 2 + cmd_text.chars().count() + 2 + hint.chars().count();
        let spacer = " ".repeat(width.saturating_sub(used).saturating_sub(6));
        let mut spans = vec![
            Span::raw(" "),
            Span::styled(pill_text, pill_style),
            Span::raw("  "),
            Span::styled(cmd_text, cmd_style),
            Span::raw(spacer),
            Span::styled(hint, theme::subtle()),
            Span::raw("  "),
        ];
        if selected {
            spans.push(Span::styled(format!(" {} ", keys::enter()), theme::keycap()));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn intent_suggestions(state: &AppState) -> Vec<IntentSuggestion> {
    let buf = state.ui.input.buffer.trim();
    if buf.is_empty() || buf.starts_with('/') {
        return Vec::new();
    }
    if state.ui.input.mode.ai_kind().is_some() {
        return Vec::new();
    }

    let mut out = vec![
        IntentSuggestion {
            mode: "SHELL",
            cmd: truncate(buf, 60),
            hint: "run in current shell".into(),
        },
        IntentSuggestion {
            mode: "BG",
            cmd: format!("bg {}", truncate(buf, 56)),
            hint: "manage as long-lived service".into(),
        },
        IntentSuggestion {
            mode: "AI",
            cmd: format!("ai: what does {} do", truncate(buf.split_whitespace().next().unwrap_or(buf), 40)),
            hint: "ask the assistant".into(),
        },
    ];

    if let Some(hist) = state
        .commands
        .records
        .iter()
        .rev()
        .find(|c| c.raw.starts_with(buf) && c.raw != buf)
    {
        out.push(IntentSuggestion {
            mode: "HIST",
            cmd: truncate(&hist.raw, 60),
            hint: "from history".into(),
        });
    }

    out
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
    // AI cards use a left-accent border; everything else uses a hairline all-around box.
    let is_ai = state.ui.input.mode.ai_kind().is_some()
        || matches!(
            state.ai.status,
            AiStatus::Running | AiStatus::Queued | AiStatus::Failed | AiStatus::Completed
        );
    let (header_spans, body, footer_hint) = build_context_card(state);
    let borders = if is_ai {
        Borders::LEFT
    } else {
        Borders::ALL
    };
    let block = Block::default()
        .borders(borders)
        .border_style(if is_ai {
            theme::accent()
        } else {
            theme::pane_border(false)
        })
        .style(theme::panel_surface(false))
        .padding(Padding::new(2, 2, 1, 1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines = vec![Line::from(header_spans)];
    lines.push(Line::raw(""));
    lines.extend(body);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inner,
    );

    // Hint line below the card (only when there is vertical room).
    if let Some(hint) = footer_hint {
        if area.y + area.height + 1 < area.y + area.height {
            return;
        }
        let hint_rect = Rect {
            x: area.x + 2,
            y: area.y + area.height,
            width: area.width.saturating_sub(2),
            height: 1,
        };
        frame.render_widget(Paragraph::new(hint), hint_rect);
    }
}

fn build_context_card(
    state: &AppState,
) -> (Vec<Span<'static>>, Vec<Line<'static>>, Option<Line<'static>>) {
    if let Some(approval) = state
        .approvals
        .pending
        .iter()
        .find(|r| matches!(r.mode, ApprovalMode::InlineReview))
    {
        let header = vec![
            Span::styled(
                format!(" REVIEW · #{} ", approval.command_id),
                theme::status_badge(theme::WARN),
            ),
            Span::raw("  "),
            Span::styled(approval.class.label().to_string(), theme::subtle()),
        ];
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
        ];
        let footer = Line::from(vec![
            Span::styled(format!(" {} ", keys::enter()), theme::keycap()),
            Span::styled(" approve    ", theme::muted()),
            Span::styled(" Esc ", theme::keycap()),
            Span::styled(" deny", theme::muted()),
        ]);
        return (header, body, Some(footer));
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
        let right_label = match state.ai.status {
            AiStatus::Running | AiStatus::Queued => "streaming…",
            AiStatus::Failed => "error",
            AiStatus::Completed => "done",
            _ => "",
        };
        let header = vec![
            Span::styled(" AI ", theme::focus_badge()),
            Span::raw("  "),
            Span::styled(
                format!("{} · model {}", provider, model),
                theme::subtle(),
            ),
            Span::raw("  "),
            Span::styled(right_label, theme::subtle()),
        ];

        let mut body: Vec<Line<'static>> = Vec::new();
        if let Some(err) = state.ai.last_error.as_ref() {
            body.push(Line::from(vec![Span::styled(
                truncate(err, 300),
                theme::error_accent(),
            )]));
        } else if let Some(resp) = state.ai.last_response.as_ref() {
            // Big serif italic opener from summary.
            body.push(Line::from(vec![Span::styled(
                truncate(&resp.summary, 160),
                theme::italic_serif(),
            )]));
            if !resp.recommendations.is_empty() {
                body.push(Line::raw(""));
                for (i, rec) in resp.recommendations.iter().take(3).enumerate() {
                    body.push(Line::from(vec![
                        Span::styled(
                            format!("{}. ", i + 1),
                            theme::subtle(),
                        ),
                        Span::styled(truncate(rec, 220), theme::muted()),
                    ]));
                }
            }
            if !resp.proposals.is_empty() {
                body.push(Line::raw(""));
                // dashed divider
                body.push(Line::from(vec![Span::styled(
                    "─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─",
                    theme::subtle(),
                )]));
                body.push(Line::raw(""));
                let mut chips: Vec<Span<'static>> =
                    vec![Span::styled("suggested actions  ", theme::subtle())];
                for p in resp.proposals.iter().take(3) {
                    chips.push(Span::styled(
                        format!(" ▸ {} ", truncate(&p.summary, 22)),
                        theme::accent(),
                    ));
                    chips.push(Span::raw("  "));
                }
                body.push(Line::from(chips));
            }
        } else if let Some(req) = state.ai.requests.last() {
            body.push(Line::from(vec![Span::styled(
                truncate(&req.prompt, 240),
                theme::muted(),
            )]));
        } else {
            body.push(Line::from(vec![Span::styled(
                "Ask Forge with live repo and runtime context.",
                theme::muted(),
            )]));
        }

        let footer = Line::from(vec![
            Span::styled(format!(" {} ", keys::enter()), theme::keycap()),
            Span::styled(" apply first action   ", theme::muted()),
            Span::styled(" Esc ", theme::keycap()),
            Span::styled(" dismiss   ", theme::muted()),
            Span::styled(format!(" {} ", keys::up_meta()), theme::keycap()),
            Span::styled(" revise question", theme::muted()),
        ]);
        return (header, body, Some(footer));
    }

    if let Some(cmd) = state.commands.records.last() {
        let when_src = cmd
            .started_at
            .or(cmd.ended_at)
            .unwrap_or_else(crate::shared::time::now_utc);
        let when = format_timestamp(when_src);
        let exit = cmd
            .exit_code
            .map(|c| format!("exit {c}"))
            .unwrap_or_else(|| status_label(cmd.status).to_string());
        let header = vec![
            Span::styled(
                format!("LAST RUN · {}", truncate(&cmd.raw, 32).to_uppercase()),
                theme::subtle(),
            ),
            Span::raw("   "),
            Span::styled(format!("{when} · {exit}"), theme::subtle()),
        ];
        let body = if let Some(output) = state
            .logs
            .recent
            .iter()
            .rev()
            .find(|e| e.command_id == Some(cmd.id))
        {
            vec![Line::from(vec![Span::styled(
                truncate(&output.raw, 260),
                theme::muted(),
            )])]
        } else {
            vec![Line::from(vec![Span::styled(
                format!(
                    "{}  ·  {}",
                    cmd.safety_class.label(),
                    cmd.cwd.display()
                ),
                theme::muted(),
            )])]
        };
        return (header, body, None);
    }

    (
        vec![Span::styled("READY", theme::subtle())],
        vec![Line::from(vec![Span::styled(
            "Shell is quiet. Press / for commands or F2 to switch modes.",
            theme::muted(),
        )])],
        None,
    )
}

fn render_recent_chips(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    if area.height == 0 {
        return;
    }
    let mut recents: Vec<String> = state
        .commands
        .records
        .iter()
        .rev()
        .take(3)
        .map(|c| truncate(&c.raw, 24))
        .collect();
    if recents.is_empty() {
        recents = vec![
            "cargo run --debug".into(),
            "git push origin main".into(),
            "bg cargo test --watch".into(),
        ];
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
