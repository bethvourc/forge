use ratatui::style::{Color, Modifier, Style};

use crate::domain::{LogSeverity, NotificationLevel, ServiceHealth, TimelineKind};

pub const BG_BASE: Color = Color::Rgb(10, 15, 23);
pub const BG_ELEVATED: Color = Color::Rgb(17, 24, 34);
pub const BG_PANEL: Color = Color::Rgb(22, 31, 44);
pub const BG_SELECTION: Color = Color::Rgb(24, 56, 64);
pub const PANEL_BORDER: Color = Color::Rgb(69, 88, 108);
pub const PANEL_FOCUS: Color = Color::Rgb(104, 212, 201);
pub const TEXT_PRIMARY: Color = Color::Rgb(232, 238, 245);
pub const TEXT_MUTED: Color = Color::Rgb(164, 179, 193);
pub const TEXT_SUBTLE: Color = Color::Rgb(111, 125, 140);
pub const ACCENT: Color = Color::Rgb(104, 212, 201);
pub const SUCCESS: Color = Color::Rgb(148, 210, 127);
pub const WARN: Color = Color::Rgb(237, 182, 74);
pub const ERROR: Color = Color::Rgb(230, 113, 106);
pub const INFO: Color = Color::Rgb(114, 167, 255);

pub fn pane_border(focused: bool) -> Style {
    Style::default().fg(if focused { PANEL_FOCUS } else { PANEL_BORDER })
}

pub fn panel_surface(focused: bool) -> Style {
    Style::default().bg(if focused { BG_PANEL } else { BG_ELEVATED })
}

pub fn panel_title(focused: bool) -> Style {
    Style::default()
        .fg(if focused { PANEL_FOCUS } else { TEXT_MUTED })
        .add_modifier(Modifier::BOLD)
}

pub fn section_title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
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
        .fg(BG_BASE)
        .bg(bg)
        .add_modifier(Modifier::BOLD)
}

pub fn keycap() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .bg(BG_ELEVATED)
        .add_modifier(Modifier::BOLD)
}

pub fn command_palette_item(selected: bool) -> Style {
    let mut style = Style::default()
        .fg(if selected { TEXT_PRIMARY } else { TEXT_MUTED })
        .bg(if selected { BG_SELECTION } else { BG_ELEVATED });
    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

pub fn command_palette_summary(selected: bool) -> Style {
    Style::default().fg(if selected { TEXT_PRIMARY } else { TEXT_SUBTLE })
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
