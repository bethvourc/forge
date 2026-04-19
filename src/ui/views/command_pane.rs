use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AiMessageRole, AiStatus, AppState, ApprovalMode};
use crate::shared::time::format_timestamp;
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let block = Block::default()
        .title(Span::styled(" Command / AI ", theme::panel_title(focused)))
        .borders(Borders::ALL)
        .border_style(theme::pane_border(focused))
        .style(theme::primary().bg(theme::BG_BASE));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(5)])
        .split(inner);

    let top = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(sections[0]);

    render_activity(frame, top[0], state);
    let history_height = (state.config.ui.history_preview as u16).clamp(4, 8);
    let body = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(history_height)])
        .split(top[1]);
    render_ai_panel(frame, body[0], state);
    render_history(frame, body[1], state);
    render_input(frame, sections[1], state, focused);
}

fn render_input(frame: &mut Frame<'_>, area: Rect, state: &AppState, focused: bool) {
    let content_width = area.width.saturating_sub(6) as usize;
    let visible_input = visible_input_text(&state.ui.input_buffer, content_width.max(8));

    let lines = vec![
        Line::from(vec![
            Span::styled(" $ ", theme::status_badge(theme::ACCENT)),
            Span::raw(" "),
            Span::styled(visible_input, theme::primary()),
            Span::styled(
                if focused { "█" } else { " " },
                if focused {
                    theme::accent()
                } else {
                    theme::subtle()
                },
            ),
        ]),
        Line::from(vec![Span::styled(
            if state.ui.input_buffer.is_empty() {
                "Start typing a shell command or use /help for slash commands."
            } else {
                "Enter runs the command. Tab changes focus. Alt+Left/Right switches dashboard tabs."
            },
            theme::muted(),
        )]),
        Line::from(vec![
            Span::styled("shell ", theme::subtle()),
            Span::styled(
                state.config.commands.default_shell.clone(),
                theme::primary(),
            ),
            Span::raw("  "),
            Span::styled("focus ", theme::subtle()),
            Span::styled(
                if focused { "command" } else { "other" },
                if focused {
                    theme::accent()
                } else {
                    theme::muted()
                },
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .title(Span::styled(" Input ", theme::section_title()))
                .borders(Borders::ALL)
                .border_style(theme::pane_border(focused)),
        ),
        area,
    );
}

fn render_activity(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let mut lines = Vec::new();

    if let Some(approval) = state
        .approvals
        .pending
        .iter()
        .find(|request| matches!(request.mode, ApprovalMode::InlineReview))
    {
        lines.push(Line::from(vec![
            Span::styled("REVIEW ", theme::status_badge(theme::WARN)),
            Span::raw(" "),
            Span::styled(
                format!(
                    "#{} [{}] {}",
                    approval.command_id,
                    approval.class.label(),
                    approval.summary
                ),
                theme::primary(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Enter", theme::accent()),
            Span::styled(" approve", theme::muted()),
            Span::raw("  "),
            Span::styled("Esc", theme::accent()),
            Span::styled(" deny", theme::muted()),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("READY ", theme::status_badge(theme::SUCCESS)),
            Span::raw(" "),
            Span::styled("No inline approvals waiting", theme::muted()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Enter", theme::accent()),
            Span::styled(" run command", theme::muted()),
            Span::raw("  "),
            Span::styled("Ctrl+L", theme::accent()),
            Span::styled(" clear input", theme::muted()),
        ]));
    }

    if let Some(notification) = state.notifications.items.last() {
        let color = theme::notification_color(notification.level);
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("LAST NOTICE ", theme::status_badge(color)),
            Span::raw(" "),
            Span::styled(
                format!(
                    "[{}] {}",
                    format_timestamp(notification.created_at),
                    notification.message
                ),
                theme::primary(),
            ),
        ]));
    } else {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("/help", theme::accent()),
            Span::styled(" for slash commands and key hints", theme::muted()),
        ]));
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title(Span::styled(" Activity ", theme::section_title())))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_history(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let mut lines = Vec::new();

    if state.commands.records.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No commands yet.", theme::muted()),
            Span::raw(" "),
            Span::styled("Try", theme::muted()),
            Span::raw(" "),
            Span::styled("pwd", theme::accent()),
            Span::raw(" "),
            Span::styled("or", theme::muted()),
            Span::raw(" "),
            Span::styled("sleep 5 &", theme::accent()),
            Span::raw(" "),
            Span::styled("to seed the dashboard.", theme::muted()),
        ]));
    }

    for record in state
        .commands
        .records
        .iter()
        .rev()
        .take(state.config.ui.history_preview.max(6))
    {
        let status_color = match record.status {
            crate::domain::CommandStatus::PendingReview
            | crate::domain::CommandStatus::PendingApproval => theme::WARN,
            crate::domain::CommandStatus::Queued | crate::domain::CommandStatus::Running => {
                theme::ACCENT
            }
            crate::domain::CommandStatus::Succeeded => theme::SUCCESS,
            crate::domain::CommandStatus::Failed | crate::domain::CommandStatus::Denied => {
                theme::ERROR
            }
            crate::domain::CommandStatus::Cancelled => theme::TEXT_SUBTLE,
        };
        let summary = if let Some(code) = record.exit_code {
            format!("exit {code}")
        } else {
            format!("{:?}", record.status).to_lowercase()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("#{} ", record.id), theme::accent()),
            Span::styled(format!("[{summary}] "), theme::status_badge(status_color)),
            Span::raw(" "),
            Span::styled(record.raw.clone(), theme::primary()),
        ]));
    }

    let block = Block::default().title(Span::styled(" Recent Commands ", theme::section_title()));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_ai_panel(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let mut lines = Vec::new();
    let (status_label, status_color) = match state.ai.status {
        AiStatus::Disabled => ("OFF", theme::TEXT_SUBTLE),
        AiStatus::Unconfigured => ("SETUP", theme::WARN),
        AiStatus::Ready => ("READY", theme::SUCCESS),
        AiStatus::Queued => ("QUEUED", theme::INFO),
        AiStatus::Running => ("RUNNING", theme::INFO),
        AiStatus::Completed => ("DONE", theme::SUCCESS),
        AiStatus::Failed => ("ERROR", theme::ERROR),
    };

    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", status_label),
            theme::status_badge(status_color),
        ),
        Span::raw(" "),
        Span::styled(
            state
                .ai
                .provider
                .clone()
                .unwrap_or_else(|| "provider:unconfigured".to_string()),
            theme::primary(),
        ),
        if let Some(model) = state.ai.model.as_ref() {
            Span::styled(format!("  model:{model}"), theme::muted())
        } else {
            Span::styled("  model:default", theme::muted())
        },
    ]));

    if let Some(request) = state.ai.requests.last() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", request.kind.label().to_ascii_uppercase()),
                theme::accent(),
            ),
            Span::styled(
                visible_input_text(&request.prompt, area.width.saturating_sub(12) as usize),
                theme::primary(),
            ),
        ]));
    }

    match state.ai.status {
        AiStatus::Running | AiStatus::Queued => {
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![Span::styled(
                "AI request is running against the configured provider.",
                theme::muted(),
            )]));
        }
        AiStatus::Failed => {
            lines.push(Line::raw(""));
            if let Some(error) = state.ai.last_error.as_ref() {
                lines.push(Line::from(vec![Span::styled(
                    truncate_multiline(error, area.width.saturating_sub(4) as usize),
                    theme::primary(),
                )]));
            }
        }
        _ => {
            if let Some(response) = state.ai.last_response.as_ref() {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    response.summary.clone(),
                    theme::primary(),
                )]));
                lines.push(Line::from(vec![Span::styled(
                    truncate_multiline(&response.message, area.width.saturating_sub(4) as usize),
                    theme::muted(),
                )]));
                if !response.recommendations.is_empty() {
                    lines.push(Line::raw(""));
                    lines.push(Line::from(vec![Span::styled(
                        "Recommendations",
                        theme::section_title(),
                    )]));
                    for recommendation in response.recommendations.iter().take(3) {
                        lines.push(Line::from(vec![
                            Span::styled("• ", theme::accent()),
                            Span::styled(
                                truncate_multiline(
                                    recommendation,
                                    area.width.saturating_sub(6) as usize,
                                ),
                                theme::primary(),
                            ),
                        ]));
                    }
                }
                if !response.citations.is_empty() {
                    lines.push(Line::raw(""));
                    lines.push(Line::from(vec![Span::styled(
                        "Grounding",
                        theme::section_title(),
                    )]));
                    for citation in response.citations.iter().take(2) {
                        lines.push(Line::from(vec![
                            Span::styled(format!("{} ", citation.label), theme::accent()),
                            Span::styled(
                                truncate_multiline(
                                    &citation.detail,
                                    area.width.saturating_sub(6) as usize,
                                ),
                                theme::muted(),
                            ),
                        ]));
                    }
                }
            } else if let Some(message) = state
                .ai
                .messages
                .iter()
                .rev()
                .find(|message| matches!(message.role, AiMessageRole::Assistant))
            {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    truncate_multiline(&message.content, area.width.saturating_sub(4) as usize),
                    theme::primary(),
                )]));
            } else {
                lines.push(Line::raw(""));
                lines.push(Line::from(vec![Span::styled(
                    "Use /ai <prompt> for assistance or /diagnose <prompt> for grounded diagnosis.",
                    theme::muted(),
                )]));
            }
        }
    }

    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title(Span::styled(" AI Response ", theme::section_title())))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn visible_input_text(input: &str, max_chars: usize) -> String {
    if input.is_empty() {
        return "Type a shell command".to_string();
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

fn truncate_multiline(value: &str, max_chars: usize) -> String {
    let single_line = value.lines().collect::<Vec<_>>().join(" ");
    visible_input_text(&single_line, max_chars.max(8))
}
