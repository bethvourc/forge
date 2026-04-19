use ratatui::style::{Color, Modifier, Style};

use crate::domain::{LogSeverity, NotificationLevel, ServiceHealth, TimelineKind};

pub const BG_BASE: Color = Color::Rgb(16, 18, 22);
pub const BG_ELEVATED: Color = Color::Rgb(22, 26, 33);
pub const PANEL_BORDER: Color = Color::Rgb(82, 92, 107);
pub const PANEL_FOCUS: Color = Color::Rgb(86, 156, 214);
pub const TEXT_PRIMARY: Color = Color::Rgb(232, 236, 241);
pub const TEXT_MUTED: Color = Color::Rgb(146, 156, 169);
pub const TEXT_SUBTLE: Color = Color::Rgb(103, 112, 124);
pub const ACCENT: Color = Color::Rgb(87, 156, 255);
pub const SUCCESS: Color = Color::Rgb(108, 196, 145);
pub const WARN: Color = Color::Rgb(242, 201, 76);
pub const ERROR: Color = Color::Rgb(235, 111, 146);
pub const INFO: Color = Color::Rgb(86, 182, 194);

pub fn pane_border(focused: bool) -> Style {
    Style::default().fg(if focused { PANEL_FOCUS } else { PANEL_BORDER })
}

pub fn panel_title(focused: bool) -> Style {
    Style::default()
        .fg(if focused { PANEL_FOCUS } else { TEXT_MUTED })
        .add_modifier(Modifier::BOLD)
}

pub fn section_title() -> Style {
    Style::default().fg(TEXT_MUTED).add_modifier(Modifier::BOLD)
}

pub fn primary() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

pub fn muted() -> Style {
    Style::default().fg(TEXT_MUTED)
}

pub fn subtle() -> Style {
    Style::default().fg(TEXT_SUBTLE)
}

pub fn accent() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn status_badge(bg: Color) -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(bg)
        .add_modifier(Modifier::BOLD)
}

pub fn notification_color(level: NotificationLevel) -> Color {
    match level {
        NotificationLevel::Info => INFO,
        NotificationLevel::Warn => WARN,
        NotificationLevel::Error => ERROR,
    }
}

pub fn health_color(health: ServiceHealth) -> Color {
    match health {
        ServiceHealth::Healthy => SUCCESS,
        ServiceHealth::Starting => INFO,
        ServiceHealth::Degraded => WARN,
        ServiceHealth::Unhealthy => ERROR,
        ServiceHealth::Stopped => TEXT_SUBTLE,
        ServiceHealth::Unknown => TEXT_MUTED,
    }
}

pub fn severity_color(severity: LogSeverity) -> Color {
    match severity {
        LogSeverity::Trace => TEXT_SUBTLE,
        LogSeverity::Debug => INFO,
        LogSeverity::Info => TEXT_PRIMARY,
        LogSeverity::Warn => WARN,
        LogSeverity::Error => ERROR,
        LogSeverity::Unknown => TEXT_MUTED,
    }
}

pub fn timeline_color(kind: TimelineKind) -> Color {
    match kind {
        TimelineKind::System => INFO,
        TimelineKind::Ai => SUCCESS,
        TimelineKind::Command => ACCENT,
        TimelineKind::Log => TEXT_PRIMARY,
        TimelineKind::Approval => WARN,
        TimelineKind::Error => ERROR,
    }
}
