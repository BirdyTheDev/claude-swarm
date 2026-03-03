use ratatui::style::{Color, Modifier, Style};
use std::sync::OnceLock;

use crate::config::settings::ThemeName;

/// Color palette for the entire UI.
#[derive(Debug, Clone)]
pub struct ThemePalette {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub accent_dim: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub working: Color,
    pub border: Color,
    pub selected_bg: Color,
    pub header_bg: Color,
    pub status_bg: Color,
}

impl ThemePalette {
    pub fn dark() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            accent: Color::Cyan,
            accent_dim: Color::DarkGray,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            working: Color::Blue,
            border: Color::DarkGray,
            selected_bg: Color::Rgb(40, 40, 60),
            header_bg: Color::Rgb(30, 30, 50),
            status_bg: Color::Rgb(25, 25, 40),
        }
    }

    pub fn light() -> Self {
        Self {
            bg: Color::White,
            fg: Color::Black,
            accent: Color::Rgb(0, 128, 170),
            accent_dim: Color::Gray,
            success: Color::Rgb(0, 140, 0),
            warning: Color::Rgb(180, 120, 0),
            error: Color::Rgb(200, 0, 0),
            working: Color::Rgb(0, 80, 180),
            border: Color::Gray,
            selected_bg: Color::Rgb(220, 225, 240),
            header_bg: Color::Rgb(230, 230, 245),
            status_bg: Color::Rgb(235, 235, 250),
        }
    }

    pub fn high_contrast() -> Self {
        Self {
            bg: Color::Black,
            fg: Color::White,
            accent: Color::Rgb(0, 255, 255),
            accent_dim: Color::Rgb(128, 128, 128),
            success: Color::Rgb(0, 255, 0),
            warning: Color::Rgb(255, 255, 0),
            error: Color::Rgb(255, 0, 0),
            working: Color::Rgb(80, 140, 255),
            border: Color::White,
            selected_bg: Color::Rgb(60, 60, 80),
            header_bg: Color::Rgb(20, 20, 40),
            status_bg: Color::Rgb(15, 15, 30),
        }
    }

    pub fn for_theme(name: ThemeName) -> Self {
        match name {
            ThemeName::Dark => Self::dark(),
            ThemeName::Light => Self::light(),
            ThemeName::HighContrast => Self::high_contrast(),
        }
    }
}

// Global palette
static PALETTE: OnceLock<ThemePalette> = OnceLock::new();

/// Initialize the global palette. Call once at startup.
pub fn init_palette(name: ThemeName) {
    let _ = PALETTE.set(ThemePalette::for_theme(name));
}

/// Re-initialize palette (for settings changes). Uses a new OnceLock is not
/// possible, so we use an interior-mutable approach via a Mutex instead.
/// Since OnceLock cannot be reset, we use a separate mutable cell.
use std::sync::Mutex;
static PALETTE_MUT: Mutex<Option<ThemePalette>> = Mutex::new(None);

pub fn set_palette(name: ThemeName) {
    let p = ThemePalette::for_theme(name);
    // Set the mutable override
    *PALETTE_MUT.lock().unwrap() = Some(p.clone());
    // Also try to set OnceLock for first-time init
    let _ = PALETTE.set(p);
}

/// Get the current palette.
pub fn palette() -> ThemePalette {
    // Check mutable override first
    if let Ok(guard) = PALETTE_MUT.lock() {
        if let Some(ref p) = *guard {
            return p.clone();
        }
    }
    PALETTE.get().cloned().unwrap_or_else(ThemePalette::dark)
}

// Convenience color accessors (backwards compatible)
pub fn bg() -> Color { palette().bg }
pub fn fg() -> Color { palette().fg }
pub fn accent() -> Color { palette().accent }
pub fn accent_dim() -> Color { palette().accent_dim }
pub fn success() -> Color { palette().success }
pub fn warning() -> Color { palette().warning }
pub fn error_color() -> Color { palette().error }
pub fn working() -> Color { palette().working }
pub fn border_color() -> Color { palette().border }
pub fn selected_bg() -> Color { palette().selected_bg }
pub fn header_bg() -> Color { palette().header_bg }
pub fn status_bg() -> Color { palette().status_bg }

// Styles
pub fn title_style() -> Style {
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
}

pub fn selected_style() -> Style {
    Style::default().bg(selected_bg()).fg(fg())
}

pub fn header_style() -> Style {
    Style::default().bg(header_bg()).fg(fg())
}

pub fn status_bar_style() -> Style {
    Style::default().bg(status_bg()).fg(fg())
}

pub fn border_style() -> Style {
    Style::default().fg(border_color())
}

pub fn agent_status_style(status: &crate::types::agent::AgentStatus) -> Style {
    use crate::types::agent::AgentStatus;
    match status {
        AgentStatus::Starting => Style::default().fg(warning()),
        AgentStatus::Idle => Style::default().fg(success()),
        AgentStatus::Working => Style::default().fg(working()),
        AgentStatus::Waiting => Style::default().fg(warning()),
        AgentStatus::Completed => Style::default().fg(success()).add_modifier(Modifier::DIM),
        AgentStatus::Failed => Style::default().fg(error_color()),
        AgentStatus::Stopped => Style::default().fg(accent_dim()),
        AgentStatus::InMeeting => Style::default().fg(accent()).add_modifier(Modifier::BOLD),
    }
}

pub fn task_status_style(status: &crate::types::task::TaskStatus) -> Style {
    use crate::types::task::TaskStatus;
    match status {
        TaskStatus::Pending => Style::default().fg(accent_dim()),
        TaskStatus::Assigned => Style::default().fg(warning()),
        TaskStatus::InProgress => Style::default().fg(working()),
        TaskStatus::Completed => Style::default().fg(success()),
        TaskStatus::Failed => Style::default().fg(error_color()),
    }
}

pub fn mode_style() -> Style {
    Style::default().fg(accent()).add_modifier(Modifier::BOLD)
}

pub fn dim_style() -> Style {
    Style::default().fg(accent_dim())
}

pub fn bold_style() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}
