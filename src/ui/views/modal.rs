use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::domain::{AppState, ModalState};

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
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::raw(""),
                Line::from(format!("Command #{}: {}", approval.command_id, approval.summary)),
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
                            .border_style(Style::default().fg(Color::Yellow)),
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
                            .border_style(Style::default().fg(Color::Red)),
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

