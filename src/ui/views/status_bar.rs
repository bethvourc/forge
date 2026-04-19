use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::domain::AppState;

pub fn render(frame: &mut Frame<'_>, area: Rect, state: &AppState) {
    let branch = state.git.branch.clone().unwrap_or_else(|| "no-branch".to_string());
    let head = state.git.head.clone().unwrap_or_else(|| "------".to_string());
    let dirty = if state.git.is_dirty { "dirty" } else { "clean" };
    let mode = if state.ui.modal.is_some() {
        "approval"
    } else if !state.approvals.pending.is_empty() {
        "review"
    } else {
        "normal"
    };
    let focus = match state.ui.focus {
        crate::domain::FocusTarget::CommandPane => "command",
        crate::domain::FocusTarget::DashboardPane => "dashboard",
        crate::domain::FocusTarget::EventStream => "events",
        crate::domain::FocusTarget::Modal => "modal",
    };
    let ai = if state.ai.provider.is_some() { "ai:on" } else { "ai:off" };

    let line = Line::from(vec![
        Span::styled(" Forge ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(format!(
            " repo:{}  branch:{}@{}  git:{}  mode:{}  focus:{}  {}  pending:{} ",
            state.project.name,
            branch,
            head,
            dirty,
            mode,
            focus,
            ai,
            state.approvals.pending.len()
        )),
    ]);

    frame.render_widget(Paragraph::new(line), area);
}

