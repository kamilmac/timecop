//! Application configuration and color themes

use ratatui::style::{Color, Modifier, Style};
use std::time::Duration;

pub use crate::theme::ThemeMode;

/// Application configuration
pub struct Config {
    pub colors: Colors,
    pub timing: Timing,
    pub theme: ThemeMode,
}

impl Default for Config {
    fn default() -> Self {
        let theme = ThemeMode::detect();
        Self {
            colors: Colors::for_theme(theme),
            timing: Timing::default(),
            theme,
        }
    }
}

/// Color palette that adapts to light/dark theme
pub struct Colors {
    pub added: Color,
    pub removed: Color,
    pub added_bg: Color,
    pub removed_bg: Color,
    pub modified: Color,
    pub renamed: Color,
    pub header: Color,
    pub muted: Color,
    pub text: Color,
    pub border: Color,
    pub border_focused: Color,
    pub status_bar: Color,
    pub status_bar_text: Color,
    pub comment: Color,
    pub comment_bg: Color,
    pub logo_primary: Color,
    pub logo_highlight: Color,
}

impl Colors {
    /// Create colors for the given theme
    pub fn for_theme(theme: ThemeMode) -> Self {
        match theme {
            ThemeMode::Dark => Self::dark(),
            ThemeMode::Light => Self::light(),
        }
    }

    /// Dark theme (Catppuccin Mocha inspired)
    fn dark() -> Self {
        Self {
            added: Color::Rgb(166, 227, 161),
            removed: Color::Rgb(243, 139, 168),
            added_bg: Color::Rgb(30, 50, 40),
            removed_bg: Color::Rgb(50, 30, 35),
            modified: Color::Rgb(250, 179, 135),
            renamed: Color::Rgb(203, 166, 247),
            header: Color::Rgb(137, 180, 250),
            muted: Color::Rgb(108, 112, 134),
            text: Color::Rgb(205, 214, 244),
            border: Color::Rgb(69, 71, 90),
            border_focused: Color::Rgb(137, 180, 250),
            status_bar: Color::Rgb(49, 50, 68),
            status_bar_text: Color::Rgb(205, 214, 244),
            comment: Color::Rgb(249, 226, 175),
            comment_bg: Color::Rgb(45, 40, 30),
            logo_primary: Color::Rgb(150, 255, 170),
            logo_highlight: Color::Rgb(255, 80, 80),
        }
    }

    /// Light theme (high contrast for light backgrounds)
    fn light() -> Self {
        Self {
            added: Color::Rgb(0, 110, 0),
            removed: Color::Rgb(180, 0, 30),
            added_bg: Color::Rgb(210, 245, 210),
            removed_bg: Color::Rgb(255, 215, 220),
            modified: Color::Rgb(160, 80, 0),
            renamed: Color::Rgb(90, 20, 180),
            header: Color::Rgb(0, 60, 180),
            muted: Color::Rgb(120, 120, 135),      // Lighter gray for dimmed text
            text: Color::Rgb(10, 10, 15),
            border: Color::Rgb(190, 195, 205),     // Dim unfocused border
            border_focused: Color::Rgb(0, 90, 220),// Bright blue focused border
            status_bar: Color::Rgb(235, 238, 245),
            status_bar_text: Color::Rgb(10, 10, 15),
            comment: Color::Rgb(160, 80, 0),
            comment_bg: Color::Rgb(255, 248, 230),
            logo_primary: Color::Rgb(0, 90, 30),
            logo_highlight: Color::Rgb(160, 0, 0),
        }
    }

    pub fn style_added(&self) -> Style {
        Style::default().fg(self.added)
    }

    pub fn style_removed(&self) -> Style {
        Style::default().fg(self.removed)
    }

    pub fn style_modified(&self) -> Style {
        Style::default().fg(self.modified)
    }

    pub fn style_muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn style_header(&self) -> Style {
        Style::default().fg(self.header).add_modifier(Modifier::BOLD)
    }

    pub fn style_selected(&self) -> Style {
        Style::default().fg(self.text).add_modifier(Modifier::BOLD)
    }

    pub fn style_border(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn style_border_focused(&self) -> Style {
        Style::default().fg(self.border_focused)
    }

    pub fn style_status_bar(&self) -> Style {
        Style::default().bg(self.status_bar).fg(self.status_bar_text)
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self::for_theme(ThemeMode::detect())
    }
}

/// Timing configuration
pub struct Timing {
    pub pr_poll_interval: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            pr_poll_interval: Duration::from_secs(120),
        }
    }
}
