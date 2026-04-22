use ratatui::style::{Color, Modifier, Style};

use crate::domain::{LogSeverity, NotificationLevel, ServiceHealth, TimelineKind};

// Editorial light palette — warm off-white canvas, near-black ink, teal accent.
// Sourced from the Forge Prompt-First hi-fi design.
pub const BG_BASE: Color = Color::Rgb(240, 238, 233); // warm off-white #f0eee9
pub const BG_ELEVATED: Color = Color::Rgb(246, 243, 236); // #f6f3ec
pub const BG_PANEL: Color = Color::Rgb(251, 249, 243); // card surface #fbf9f3
pub const BG_PANEL_FOCUS: Color = Color::Rgb(251, 249, 243);
pub const BG_CHROME: Color = Color::Rgb(235, 231, 221);
pub const BG_SHADOW: Color = Color::Rgb(229, 225, 216);
pub const BG_SELECTION: Color = Color::Rgb(221, 237, 232); // teal-wash
pub const PANEL_BORDER: Color = Color::Rgb(45, 157, 143);
pub const PANEL_BORDER_SUBTLE: Color = Color::Rgb(220, 216, 205); // hair
pub const PROMPT_BORDER: Color = Color::Rgb(222, 218, 207);
pub const PANEL_FOCUS: Color = Color::Rgb(45, 157, 143);
pub const TEXT_PRIMARY: Color = Color::Rgb(29, 29, 27); // near-black ink
pub const TEXT_MUTED: Color = Color::Rgb(74, 74, 70);
pub const TEXT_SUBTLE: Color = Color::Rgb(140, 140, 136); // faint
pub const ACCENT: Color = Color::Rgb(45, 157, 143); // teal #2d9d8f
pub const ACCENT_SOFT: Color = Color::Rgb(221, 237, 232);
pub const SUCCESS: Color = Color::Rgb(45, 157, 143);
pub const WARN: Color = Color::Rgb(200, 138, 46); // #c88a2e
pub const ERROR: Color = Color::Rgb(179, 62, 46);
pub const INFO: Color = Color::Rgb(66, 104, 161);

pub fn app_surface() -> Style {
    Style::default().bg(BG_BASE).fg(TEXT_PRIMARY)
}

pub fn chrome_surface() -> Style {
    Style::default().bg(BG_BASE).fg(TEXT_PRIMARY)
}

pub fn pane_border(focused: bool) -> Style {
    Style::default().fg(if focused {
        PANEL_FOCUS
    } else {
        PANEL_BORDER_SUBTLE
    })
}

pub fn panel_surface(focused: bool) -> Style {
    Style::default()
        .bg(if focused { BG_PANEL_FOCUS } else { BG_PANEL })
        .fg(TEXT_PRIMARY)
}

pub fn prompt_surface() -> Style {
    Style::default().bg(BG_PANEL).fg(TEXT_PRIMARY)
}

pub fn prompt_border() -> Style {
    Style::default().fg(PROMPT_BORDER)
}

pub fn prompt_shadow() -> Style {
    Style::default().bg(BG_SHADOW)
}

pub fn prompt_question() -> Style {
    Style::default()
        .fg(TEXT_MUTED)
        .add_modifier(Modifier::ITALIC | Modifier::BOLD)
}

pub fn prompt_placeholder() -> Style {
    Style::default().fg(TEXT_SUBTLE).add_modifier(Modifier::DIM)
}

pub fn prompt_cursor(dimmed: bool) -> Style {
    if dimmed {
        subtle()
    } else {
        Style::default()
            .fg(TEXT_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }
}

pub fn separator() -> Style {
    Style::default().fg(PANEL_BORDER_SUBTLE).bg(BG_BASE)
}

pub fn status_context() -> Style {
    Style::default().fg(TEXT_SUBTLE)
}

pub fn panel_title(focused: bool) -> Style {
    Style::default()
        .fg(if focused { TEXT_PRIMARY } else { TEXT_MUTED })
        .add_modifier(Modifier::ITALIC)
}

pub fn section_title() -> Style {
    Style::default()
        .fg(TEXT_SUBTLE)
        .add_modifier(Modifier::BOLD)
}

pub fn label() -> Style {
    Style::default().fg(TEXT_SUBTLE)
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

pub fn italic_serif() -> Style {
    Style::default()
        .fg(TEXT_PRIMARY)
        .add_modifier(Modifier::ITALIC)
}

pub fn italic_muted() -> Style {
    Style::default()
        .fg(TEXT_MUTED)
        .add_modifier(Modifier::ITALIC)
}

pub fn brand_badge() -> Style {
    pill(Color::White, ACCENT)
}

pub fn status_badge(bg: Color) -> Style {
    pill(Color::White, bg)
}

pub fn quiet_badge() -> Style {
    Style::default().fg(TEXT_MUTED).bg(BG_ELEVATED)
}

pub fn focus_badge() -> Style {
    Style::default()
        .fg(Color::White)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD)
}

pub fn pill(fg: Color, bg: Color) -> Style {
    Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
}

pub fn keycap() -> Style {
    Style::default().fg(TEXT_MUTED).bg(BG_ELEVATED)
}

pub fn command_palette_item(selected: bool) -> Style {
    let mut style = Style::default()
        .fg(if selected { TEXT_PRIMARY } else { TEXT_MUTED })
        .bg(if selected { ACCENT_SOFT } else { BG_PANEL });
    if selected {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

pub fn command_palette_summary(selected: bool) -> Style {
    Style::default().fg(if selected { TEXT_MUTED } else { TEXT_SUBTLE })
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
        TimelineKind::Ai => ACCENT,
        TimelineKind::Command => TEXT_PRIMARY,
        TimelineKind::Log => TEXT_MUTED,
        TimelineKind::Approval => WARN,
        TimelineKind::Error => ERROR,
    }
}
