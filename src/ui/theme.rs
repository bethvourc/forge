use ratatui::style::{Color, Modifier, Style};

use crate::domain::{LogSeverity, NotificationLevel, ServiceHealth, TimelineKind};

pub const BG_BASE: Color = Color::Rgb(7, 11, 17);
pub const BG_ELEVATED: Color = Color::Rgb(12, 17, 25);
pub const BG_PANEL: Color = Color::Rgb(16, 22, 32);
pub const BG_PANEL_FOCUS: Color = Color::Rgb(20, 27, 39);
pub const BG_CHROME: Color = Color::Rgb(24, 31, 44);
pub const BG_SELECTION: Color = Color::Rgb(30, 44, 58);
pub const PANEL_BORDER: Color = Color::Rgb(60, 73, 91);
pub const PANEL_BORDER_SUBTLE: Color = Color::Rgb(40, 50, 64);
pub const PANEL_FOCUS: Color = Color::Rgb(122, 220, 198);
pub const TEXT_PRIMARY: Color = Color::Rgb(236, 240, 244);
pub const TEXT_MUTED: Color = Color::Rgb(170, 181, 193);
pub const TEXT_SUBTLE: Color = Color::Rgb(108, 121, 137);
pub const ACCENT: Color = Color::Rgb(122, 220, 198);
pub const SUCCESS: Color = Color::Rgb(150, 216, 138);
pub const WARN: Color = Color::Rgb(236, 190, 90);
pub const ERROR: Color = Color::Rgb(229, 113, 98);
pub const INFO: Color = Color::Rgb(119, 167, 255);

pub fn app_surface() -> Style {
    Style::default().bg(BG_BASE)
}

pub fn chrome_surface() -> Style {
    Style::default().bg(BG_ELEVATED)
}

pub fn pane_border(focused: bool) -> Style {
    Style::default().fg(if focused {
        PANEL_FOCUS
    } else {
        PANEL_BORDER_SUBTLE
    })
}

pub fn panel_surface(focused: bool) -> Style {
    Style::default().bg(if focused { BG_PANEL_FOCUS } else { BG_PANEL })
}

pub fn panel_title(focused: bool) -> Style {
    Style::default()
        .fg(if focused { TEXT_PRIMARY } else { TEXT_MUTED })
        .add_modifier(Modifier::BOLD)
}

pub fn section_title() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

pub fn label() -> Style {
    Style::default()
        .fg(TEXT_SUBTLE)
        .add_modifier(Modifier::BOLD)
}

pub fn primary() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

pub fn primary_emphasis() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .add_modifier(Modifier::BOLD)
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

pub fn warn_accent() -> Style {
    Style::default().fg(WARN).add_modifier(Modifier::BOLD)
}

pub fn error_accent() -> Style {
    Style::default().fg(ERROR).add_modifier(Modifier::BOLD)
}

pub fn brand_badge() -> Style {
    pill(BG_BASE, ACCENT)
}

pub fn status_badge(bg: Color) -> Style {
    pill(BG_BASE, bg)
}

pub fn quiet_badge() -> Style {
    pill(TEXT_MUTED, BG_CHROME)
}

pub fn focus_badge() -> Style {
    pill(PANEL_FOCUS, BG_CHROME)
}

pub fn pill(fg: Color, bg: Color) -> Style {
    Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
}

pub fn keycap() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .bg(BG_CHROME)
        .add_modifier(Modifier::BOLD)
}

pub fn command_palette_item(selected: bool) -> Style {
    let mut style = Style::default()
        .fg(if selected { TEXT_PRIMARY } else { TEXT_MUTED })
        .bg(if selected { BG_SELECTION } else { BG_PANEL });
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
