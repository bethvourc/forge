mod command_pane;
mod dashboard;
mod event_stream;
mod modal;
mod status_bar;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::domain::{AppState, FocusTarget};
use crate::ui::theme;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    frame.render_widget(Block::default().style(theme::app_surface()), frame.area());

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(12),
            Constraint::Length(state.config.ui.event_stream_height),
        ])
        .split(frame.area());

    status_bar::render(frame, layout[0], state);

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(state.config.ui.main_split_pct),
            Constraint::Percentage(100 - state.config.ui.main_split_pct),
        ])
        .spacing(1)
        .split(layout[1]);

    command_pane::render(
        frame,
        main[0],
        state,
        matches!(state.ui.focus, FocusTarget::CommandPane),
    );
    dashboard::render(
        frame,
        main[1],
        state,
        matches!(state.ui.focus, FocusTarget::DashboardPane),
    );
    event_stream::render(
        frame,
        layout[2],
        state,
        matches!(state.ui.focus, FocusTarget::EventStream),
    );
    modal::render(frame, state);
}
