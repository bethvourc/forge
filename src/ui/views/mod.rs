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
            Constraint::Length(3), // top identity bar + hairline
            Constraint::Min(8),    // main stage
            Constraint::Length(2), // bottom activity rail + hairline
        ])
        .split(frame.area());

    status_bar::render(frame, layout[0], state);

    let ops_open = matches!(
        state.ui.focus,
        FocusTarget::DashboardPane | FocusTarget::EventStream
    );

    if ops_open {
        // Drawer-over-prompt layout: left 48% dimmed prompt, right 52% ops.
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(48), Constraint::Percentage(52)])
            .split(layout[1]);
        command_pane::render(frame, main[0], state, false);
        dashboard::render(
            frame,
            main[1],
            state,
            matches!(state.ui.focus, FocusTarget::DashboardPane),
        );
    } else {
        command_pane::render(
            frame,
            layout[1],
            state,
            matches!(state.ui.focus, FocusTarget::CommandPane),
        );
    }

    event_stream::render(frame, layout[2], state, false);
    modal::render(frame, state);
}
