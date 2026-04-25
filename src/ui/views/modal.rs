use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, CommandProvenance, ModalState};
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let Some(modal) = state.ui.modal.as_ref() else {
        return;
    };

    let area = centered_rect(70, 40, frame.area());
    frame.render_widget(Clear, area);

    match modal {
        ModalState::Approval(approval_id) => {
            let approval = state
                .approvals
                .pending
                .iter()
                .find(|request| request.id == *approval_id);
            let Some(approval) = approval else {
                return;
            };

            let lines = vec![
                Line::from(Span::styled(
                    format!("{} Action", approval.class.label()),
                    Style::default()
                        .fg(theme::WARN)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
                Line::from(vec![
                    Span::styled(
                        format!("Command #{} ", approval.command_id),
                        theme::accent(),
                    ),
                    Span::styled(
                        format!("[{}] ", approval.class.label()),
                        theme::status_badge(theme::WARN),
                    ),
                    Span::styled(
                        format!("[{}] ", provenance_label(approval.execution.provenance)),
                        theme::status_badge(theme::INFO),
                    ),
                    Span::styled(approval.summary.clone(), theme::primary()),
                ]),
                Line::from(vec![
                    Span::styled("cwd ", theme::subtle()),
                    Span::styled(
                        approval.execution.cwd.display().to_string(),
                        theme::primary(),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("command ", theme::subtle()),
                    Span::styled(approval.execution.raw.clone(), theme::primary()),
                ]),
                Line::from(approval.detail.clone()),
                Line::raw(""),
                Line::from("Press Enter or y to approve."),
                Line::from("Press Esc or n to deny."),
            ];

            frame.render_widget(
                Paragraph::new(lines)
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title("Approval Required")
                            .borders(Borders::ALL)
                            .border_style(theme::pane_border(true))
                            .style(theme::panel_surface(true)),
                    ),
                area,
            );
        }
        ModalState::Help => {
            let lines = vec![
                Line::from(Span::styled(
                    "Forge Commands",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
                Line::from("/help              open this help"),
                Line::from("/ai <prompt>       queue an AI assistance request"),
                Line::from("/diagnose <prompt> queue an AI diagnosis request"),
                Line::from("/apply <n>         run AI proposal n through safety checks"),
                Line::from("/bg <cmd>          run a background command"),
                Line::from("/cancel <id>       cancel a running command"),
                Line::from("/history           show recent shell command history"),
                Line::from("/rerun <id>        replay command id through safety checks"),
                Line::from("/rerun-last        replay the latest shell history entry"),
                Line::from("/approve           approve the current pending action"),
                Line::from("/deny              deny the current pending action"),
                Line::from("cd <path>          update Forge session cwd"),
                Line::from("export K=V         set a Forge session env value"),
                Line::from("unset K            remove a Forge session env value"),
                Line::from("/tab next          next dashboard tab"),
                Line::from("/tab prev          previous dashboard tab"),
                Line::from("/clear             clear input and visible logs"),
                Line::from("/quit              exit Forge"),
                Line::raw(""),
                Line::from("Keyboard"),
                Line::from("Tab                cycle pane focus"),
                Line::from("Tab (with /)       insert selected slash command"),
                Line::from("Up/Down (with /)   move through slash suggestions"),
                Line::from("F2                 cycle shell / AI assist / AI diagnose"),
                Line::from("Ctrl+P / Ctrl+N    previous / next history for the current mode"),
                Line::from("Ctrl+J             insert a newline in the console editor"),
                Line::from("Left/Right         switch dashboard tabs when dashboard is focused"),
                Line::from("Enter              run input or confirm modal"),
                Line::from("Esc                close modal / deny approval"),
                Line::from("F1                 open help"),
            ];

            frame.render_widget(
                Paragraph::new(lines)
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title("Help")
                            .borders(Borders::ALL)
                            .border_style(theme::pane_border(true))
                            .style(theme::panel_surface(true)),
                    ),
                area,
            );
        }
        ModalState::History => {
            let mut lines = vec![
                Line::from(Span::styled(
                    "Command History",
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
            ];

            if state.commands.history.is_empty() {
                lines.push(Line::from("No shell history recorded yet."));
            } else {
                lines.push(Line::from(vec![
                    Span::styled("id", theme::subtle()),
                    Span::raw("    "),
                    Span::styled("status", theme::subtle()),
                    Span::raw("      "),
                    Span::styled("cwd", theme::subtle()),
                    Span::raw("    "),
                    Span::styled("command", theme::subtle()),
                ]));
                lines.push(Line::raw(""));

                for entry in state.commands.history.iter().rev().take(12) {
                    let id = entry
                        .command_id
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "-".to_string());
                    let status = status_label(entry.status);
                    lines.push(Line::from(vec![
                        Span::styled(format!("{id:<4}"), theme::accent()),
                        Span::styled(format!("{status:<10}"), theme::muted()),
                        Span::styled(
                            format!("{:<18}", truncate_path(&entry.cwd, 18)),
                            theme::subtle(),
                        ),
                        Span::styled(truncate(&entry.raw, 72), theme::primary()),
                    ]));
                }

                lines.push(Line::raw(""));
                lines.push(Line::from(
                    "/rerun <id> replays a command with approval checks.",
                ));
                lines.push(Line::from("/rerun-last replays the newest history entry."));
            }

            frame.render_widget(
                Paragraph::new(lines)
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title("History")
                            .borders(Borders::ALL)
                            .border_style(theme::pane_border(true))
                            .style(theme::panel_surface(true)),
                    ),
                area,
            );
        }
        ModalState::Error(message) => {
            frame.render_widget(
                Paragraph::new(message.clone())
                    .alignment(Alignment::Left)
                    .wrap(Wrap { trim: false })
                    .block(
                        Block::default()
                            .title("Error")
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(theme::ERROR))
                            .style(theme::panel_surface(true)),
                    ),
                area,
            );
        }
    }
}

fn provenance_label(provenance: CommandProvenance) -> &'static str {
    match provenance {
        CommandProvenance::UserInput => "user",
        CommandProvenance::SlashCommand => "slash",
        CommandProvenance::ApprovalEscalation => "approval",
        CommandProvenance::AiSuggestion => "ai",
    }
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
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let kept = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    format!("{kept}…")
}

fn truncate_path(path: &std::path::Path, max_chars: usize) -> String {
    truncate(&path.display().to_string(), max_chars)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
