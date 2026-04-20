use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, ModalState};
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
                Line::from(format!(
                    "Command #{}: {}",
                    approval.command_id, approval.summary
                )),
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
                            .border_style(theme::pane_border(true)),
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
                Line::from("/approve           approve the current pending action"),
                Line::from("/deny              deny the current pending action"),
                Line::from("/tab next          next dashboard tab"),
                Line::from("/tab prev          previous dashboard tab"),
                Line::from("/quit              exit Forge"),
                Line::raw(""),
                Line::from("Keyboard"),
                Line::from("Tab                cycle pane focus"),
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
                            .border_style(theme::pane_border(true)),
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
                            .border_style(Style::default().fg(theme::ERROR)),
                    ),
                area,
            );
        }
    }
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
