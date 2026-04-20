use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::commands::{slash_command_suggestions, SlashCommandSpec};
use crate::domain::{AiStatus, AppState, ApprovalMode, CommandProvenance, CommandStatus};
use crate::shared::time::format_timestamp;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let block = Block::default()
        .title(Span::styled(" Command Deck ", theme::panel_title(focused)))
        .borders(Borders::ALL)
        .border_style(theme::pane_border(focused))
        .style(theme::panel_surface(focused))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let slash_suggestions = slash_command_suggestions(&state.ui.input.buffer);
    let context_lines = build_context_lines(state, inner.width.saturating_sub(4) as usize);
    let context_height = context_lines.len().clamp(2, 5) as u16;

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(context_height), Constraint::Min(0)])
        .split(inner);

    render_context_strip(frame, sections[0], context_lines);
    render_console(frame, sections[1], state, &slash_suggestions);
}

fn render_context_strip(frame: &mut Frame<'_>, area: Rect, lines: Vec<Line<'static>>) {
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(theme::pane_border(false))
                    .style(theme::chrome_surface()),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_console(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    slash_suggestions: &[&SlashCommandSpec],
) {
    let mode = state.ui.input.mode;
    let slash_mode = state.ui.input.buffer.trim_start().starts_with('/');
    let mode_label: &'static str = if slash_mode { "SLASH" } else { mode.label() };
    let mode_color = if slash_mode || mode.ai_kind().is_some() {
        theme::INFO
    } else {
        theme::ACCENT
    };
    let visible_suggestions = visible_command_suggestions(slash_suggestions, state);
    let suggestion_rows = if visible_suggestions.is_empty() {
        0
    } else {
        2 + visible_suggestions.len() as u16
    };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(suggestion_rows),
            Constraint::Length(1),
        ])
        .split(area);

    let editor_width = sections[1].width.saturating_sub(12) as usize;
    let editor_lines = visible_editor_lines(
        &state.ui.input.buffer,
        state.ui.input.cursor,
        editor_width.max(12),
        sections[1].height.max(1) as usize,
        editor_gutter_label(mode_label),
        editor_prompt_symbol(mode, slash_mode),
    );

    frame.render_widget(
        Paragraph::new(editor_header_lines(
            mode_label,
            mode_color,
            slash_mode,
            mode.ai_kind().is_some(),
            state.ui.input.buffer.is_empty(),
            !visible_suggestions.is_empty(),
        ))
        .style(theme::primary())
        .wrap(Wrap { trim: false }),
        sections[0],
    );

    frame.render_widget(
        Paragraph::new(editor_lines)
            .style(theme::primary())
            .wrap(Wrap { trim: false }),
        sections[1],
    );

    if !visible_suggestions.is_empty() {
        frame.render_widget(
            Paragraph::new(suggestion_lines(
                &visible_suggestions,
                area.width.saturating_sub(26) as usize,
            ))
            .style(theme::primary())
            .wrap(Wrap { trim: false }),
            sections[2],
        );
    }

    frame.render_widget(
        Paragraph::new(console_footer_line(mode_label))
            .style(theme::primary())
            .wrap(Wrap { trim: false }),
        sections[3],
    );
}

fn build_context_lines(state: &AppState, width: usize) -> Vec<Line<'static>> {
    if let Some(approval) = state
        .approvals
        .pending
        .iter()
        .find(|request| matches!(request.mode, ApprovalMode::InlineReview))
    {
        return vec![
            Line::from(vec![
                Span::styled(" REVIEW ", theme::status_badge(theme::WARN)),
                Span::raw(" "),
                Span::styled(format!("#{} ", approval.command_id), theme::accent()),
                Span::styled(approval.summary.clone(), theme::primary()),
            ]),
            Line::from(vec![Span::styled(
                truncate_multiline(&approval.detail, width.max(12)),
                theme::muted(),
            )]),
            Line::from(vec![
                Span::styled("Enter", theme::keycap()),
                Span::styled(" approve", theme::muted()),
                Span::raw("  "),
                Span::styled("Esc", theme::keycap()),
                Span::styled(" deny", theme::muted()),
                Span::raw("  "),
                Span::styled(
                    format!(
                        "{} / {}",
                        approval.class.label(),
                        provenance_label(approval.execution.provenance)
                    ),
                    theme::subtle(),
                ),
            ]),
        ];
    }

    if state.ui.input.mode.ai_kind().is_some()
        || matches!(
            state.ai.status,
            AiStatus::Queued | AiStatus::Running | AiStatus::Failed
        )
    {
        return build_ai_context_lines(state, width);
    }

    if let Some(command) = state.commands.records.last() {
        let status_color = command_status_color(command.status);
        let latest_output = state
            .logs
            .recent
            .iter()
            .rev()
            .find(|entry| entry.command_id == Some(command.id));

        let mut lines = vec![Line::from(vec![
            Span::styled(
                format!(" {} ", command_status_label(command.status)),
                theme::status_badge(status_color),
            ),
            Span::raw(" "),
            Span::styled(format!("#{} ", command.id), theme::accent()),
            Span::styled(
                truncate_multiline(&command.raw, width.max(12)),
                theme::primary(),
            ),
        ])];

        if let Some(output) = latest_output {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("[{}] ", format_timestamp(output.ts)),
                    theme::subtle(),
                ),
                Span::styled(
                    truncate_multiline(&output.raw, width.saturating_sub(16).max(12)),
                    theme::muted(),
                ),
            ]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "{} action from {} in {}",
                    command.safety_class.label(),
                    provenance_label(command.provenance),
                    command.cwd.display()
                ),
                theme::muted(),
            )]));
        }

        return lines;
    }

    vec![
        Line::from(vec![
            Span::styled(" READY ", theme::status_badge(theme::SUCCESS)),
            Span::raw(" "),
            Span::styled(
                "Shell is quiet. This deck is ready for commands, AI, or approvals.",
                theme::primary(),
            ),
        ]),
        Line::from(vec![Span::styled(
            "Start with /help, /bg cargo run, or press F2 to switch into assist or diagnose mode.",
            theme::muted(),
        )]),
    ]
}

fn build_ai_context_lines(state: &AppState, width: usize) -> Vec<Line<'static>> {
    let (status_label, status_color) = match state.ai.status {
        AiStatus::Disabled => ("OFF", theme::TEXT_SUBTLE),
        AiStatus::Unconfigured => ("SETUP", theme::WARN),
        AiStatus::Ready => ("READY", theme::SUCCESS),
        AiStatus::Queued => ("QUEUED", theme::INFO),
        AiStatus::Running => ("RUNNING", theme::INFO),
        AiStatus::Completed => ("DONE", theme::SUCCESS),
        AiStatus::Failed => ("ERROR", theme::ERROR),
    };

    let provider = state
        .ai
        .provider
        .clone()
        .unwrap_or_else(|| "provider:unconfigured".to_string());
    let model = state
        .ai
        .model
        .clone()
        .unwrap_or_else(|| "default".to_string());

    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(" {} ", status_label),
            theme::status_badge(status_color),
        ),
        Span::raw(" "),
        Span::styled(provider, theme::primary()),
        Span::styled(format!("  model:{model}"), theme::muted()),
    ])];

    match state.ai.status {
        AiStatus::Running | AiStatus::Queued => {
            if let Some(request) = state.ai.requests.last() {
                lines.push(Line::from(vec![Span::styled(
                    truncate_multiline(&request.prompt, width.max(12)),
                    theme::primary(),
                )]));
            }
        }
        AiStatus::Failed => {
            if let Some(error) = state.ai.last_error.as_ref() {
                lines.push(Line::from(vec![Span::styled(
                    truncate_multiline(error, width.max(12)),
                    theme::muted(),
                )]));
            }
        }
        _ => {
            if let Some(response) = state.ai.last_response.as_ref() {
                lines.push(Line::from(vec![Span::styled(
                    truncate_multiline(&response.summary, width.max(12)),
                    theme::primary(),
                )]));
                if !response.recommendations.is_empty() || !response.proposals.is_empty() {
                    lines.push(Line::from(vec![Span::styled(
                        format!(
                            "{} recommendation(s)  {} proposal(s)",
                            response.recommendations.len(),
                            response.proposals.len()
                        ),
                        theme::muted(),
                    )]));
                }
            } else if state.ui.input.mode.ai_kind().is_some() {
                lines.push(Line::from(vec![Span::styled(
                    "Plain text in this mode submits directly to AI. Use / for Forge commands.",
                    theme::muted(),
                )]));
            }
        }
    }

    lines
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
    let visible_len = slash_suggestions.len().min(4);
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

fn visible_editor_lines(
    buffer: &str,
    cursor: usize,
    width: usize,
    max_lines: usize,
    gutter_label: &str,
    prompt_symbol: &str,
) -> Vec<Line<'static>> {
    let mut logical_lines = if buffer.is_empty() {
        vec![String::new()]
    } else {
        buffer
            .split('\n')
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
    if logical_lines.is_empty() {
        logical_lines.push(String::new());
    }

    let cursor_line = buffer[..cursor].chars().filter(|ch| *ch == '\n').count();
    let visible_count = max_lines.max(1).min(logical_lines.len());
    let start = cursor_line.saturating_sub(visible_count.saturating_sub(1));
    let end = (start + visible_count).min(logical_lines.len());

    logical_lines[start..end]
        .iter()
        .enumerate()
        .map(|(offset, line)| {
            let line_index = start + offset;
            let is_cursor_line = line_index == cursor_line;
            let cursor_col = if is_cursor_line {
                Some(
                    buffer[..cursor]
                        .rsplit('\n')
                        .next()
                        .unwrap_or_default()
                        .chars()
                        .count(),
                )
            } else {
                None
            };
            let rendered = render_editor_segment(line, width, cursor_col);
            Line::from(vec![
                Span::styled(
                    if line_index == start {
                        format!("{gutter_label:>7} {prompt_symbol} ")
                    } else {
                        "          ".to_string()
                    },
                    if is_cursor_line {
                        theme::accent()
                    } else {
                        theme::subtle()
                    },
                ),
                Span::styled(rendered, theme::primary()),
            ])
        })
        .collect()
}

fn render_editor_segment(line: &str, width: usize, cursor_col: Option<usize>) -> String {
    let chars = line.chars().collect::<Vec<_>>();
    let cursor_col = cursor_col.unwrap_or(chars.len());
    let visible_width = width.max(8).saturating_sub(1);
    let start = cursor_col.saturating_sub(visible_width.saturating_sub(1));
    let end = (start + visible_width).min(chars.len());
    let mut rendered = chars[start..end].iter().collect::<String>();

    if line.is_empty() && cursor_col == 0 {
        rendered.push('█');
    } else if cursor_col >= start && cursor_col <= end {
        let insert_at = chars[start..cursor_col.min(chars.len())]
            .iter()
            .collect::<String>()
            .len();
        rendered.insert(insert_at, '█');
    }

    if start > 0 {
        rendered.insert(0, '…');
    }
    if end < chars.len() {
        rendered.push('…');
    }
    rendered
}

fn truncate_multiline(value: &str, max_chars: usize) -> String {
    let single_line = value.lines().collect::<Vec<_>>().join(" ");
    visible_input_text(&single_line, max_chars.max(8))
}

fn visible_input_text(input: &str, max_chars: usize) -> String {
    if input.is_empty() {
        return String::new();
    }

    let chars = input.chars().collect::<Vec<_>>();
    if chars.len() <= max_chars {
        return input.to_string();
    }

    let tail = chars[chars.len().saturating_sub(max_chars - 1)..]
        .iter()
        .collect::<String>();
    format!("…{tail}")
}

fn provenance_label(provenance: CommandProvenance) -> &'static str {
    match provenance {
        CommandProvenance::UserInput => "user",
        CommandProvenance::SlashCommand => "slash",
        CommandProvenance::ApprovalEscalation => "approval",
        CommandProvenance::AiSuggestion => "ai",
    }
}

fn command_status_color(status: CommandStatus) -> ratatui::style::Color {
    match status {
        CommandStatus::PendingReview | CommandStatus::PendingApproval => theme::WARN,
        CommandStatus::Queued | CommandStatus::Running => theme::INFO,
        CommandStatus::Succeeded => theme::SUCCESS,
        CommandStatus::Failed | CommandStatus::Denied => theme::ERROR,
        CommandStatus::Cancelled => theme::TEXT_SUBTLE,
    }
}

fn command_status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::PendingReview => "REVIEW",
        CommandStatus::PendingApproval => "PENDING",
        CommandStatus::Queued => "QUEUED",
        CommandStatus::Running => "RUNNING",
        CommandStatus::Succeeded => "DONE",
        CommandStatus::Failed => "FAILED",
        CommandStatus::Cancelled => "CANCELLED",
        CommandStatus::Denied => "DENIED",
    }
}

fn editor_header_lines(
    mode_label: &str,
    mode_color: ratatui::style::Color,
    slash_mode: bool,
    ai_mode: bool,
    input_empty: bool,
    showing_suggestions: bool,
) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(format!(" {mode_label} "), theme::status_badge(mode_color)),
            Span::raw(" "),
            Span::styled(
                if slash_mode {
                    "Search Forge actions without leaving the command deck."
                } else if ai_mode {
                    "Ask Forge with live repo and runtime context."
                } else {
                    "Run commands directly in the current shell."
                },
                theme::muted(),
            ),
        ]),
        Line::from(vec![Span::styled(
            if showing_suggestions {
                "Arrow keys move the selector. Tab inserts. Enter runs immediately."
            } else if input_empty {
                "Pinned input keeps you oriented. Press / for actions or F2 to change modes."
            } else {
                "Multiline input is supported. History stays scoped to the current mode."
            },
            theme::subtle(),
        )]),
    ]
}

fn suggestion_lines(
    suggestions: &[(bool, &SlashCommandSpec)],
    summary_width: usize,
) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled(" Slash Actions ", theme::quiet_badge()),
            Span::raw(" "),
            Span::styled("Tab inserts the selected action", theme::subtle()),
        ]),
        Line::raw(""),
    ];

    for (selected, suggestion) in suggestions {
        lines.push(Line::from(vec![
            Span::styled(
                if *selected { "▶ " } else { "  " },
                if *selected {
                    theme::accent()
                } else {
                    theme::subtle()
                },
            ),
            Span::styled(
                format!("{:<18}", suggestion.usage),
                theme::command_palette_item(*selected),
            ),
            Span::raw(" "),
            Span::styled(
                truncate_multiline(suggestion.summary, summary_width.max(12)),
                theme::command_palette_summary(*selected),
            ),
        ]));
    }

    lines
}

fn console_footer_line(mode_label: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled("mode", theme::label()),
        Span::raw(" "),
        Span::styled(mode_label, theme::primary()),
        Span::raw("  "),
        Span::styled("F2", theme::keycap()),
        Span::styled(" cycle", theme::muted()),
        Span::raw("  "),
        Span::styled("Ctrl+P/N", theme::keycap()),
        Span::styled(" history", theme::muted()),
        Span::raw("  "),
        Span::styled("Ctrl+J", theme::keycap()),
        Span::styled(" newline", theme::muted()),
    ])
}

fn editor_gutter_label(mode_label: &str) -> &'static str {
    match mode_label {
        "SHELL" => "shell",
        "AI" => "assist",
        "DIAG" => "diagnose",
        "SLASH" => "slash",
        _ => "input",
    }
}

fn editor_prompt_symbol(mode: crate::domain::InputMode, slash_mode: bool) -> &'static str {
    if slash_mode {
        "/"
    } else {
        match mode {
            crate::domain::InputMode::Shell => "$",
            crate::domain::InputMode::AiAssist | crate::domain::InputMode::AiDiagnose => ">",
        }
    }
}
