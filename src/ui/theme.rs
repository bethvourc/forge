use ratatui::style::{Color, Modifier, Style};

use crate::domain::{LogSeverity, NotificationLevel, ServiceHealth, TimelineKind};

pub const BG_BASE: Color = Color::Rgb(14, 16, 18);
pub const BG_ELEVATED: Color = Color::Rgb(22, 25, 27);
pub const BG_PANEL: Color = Color::Rgb(28, 31, 35);
pub const BG_SELECTION: Color = Color::Rgb(63, 47, 24);
pub const PANEL_BORDER: Color = Color::Rgb(104, 95, 79);
pub const PANEL_FOCUS: Color = Color::Rgb(224, 167, 77);
pub const TEXT_PRIMARY: Color = Color::Rgb(240, 234, 223);
pub const TEXT_MUTED: Color = Color::Rgb(176, 166, 148);
pub const TEXT_SUBTLE: Color = Color::Rgb(118, 111, 99);
pub const ACCENT: Color = Color::Rgb(224, 167, 77);
pub const SUCCESS: Color = Color::Rgb(141, 185, 118);
pub const WARN: Color = Color::Rgb(238, 198, 96);
pub const ERROR: Color = Color::Rgb(221, 117, 96);
pub const INFO: Color = Color::Rgb(116, 183, 170);

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
    Style::default()
        .fg(ACCENT)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
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
        .bg(BG_PANEL)
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
