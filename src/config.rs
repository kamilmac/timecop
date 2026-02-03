use ratatui::style::{Color, Modifier, Style};
use std::time::Duration;

/// Theme mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    /// Detect theme mode from system/environment
    pub fn detect() -> Self {
        // 1. Check explicit override via TIMECOP_THEME env var
        if let Ok(theme) = std::env::var("TIMECOP_THEME") {
            match theme.to_lowercase().as_str() {
                "light" => return Self::Light,
                "dark" => return Self::Dark,
                _ => {}
            }
        }

        // 2. Check macOS system preference
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("defaults")
                .args(["read", "-g", "AppleInterfaceStyle"])
                .output()
            {
                // "Dark" means dark mode is enabled; error/empty means light mode
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.trim().eq_ignore_ascii_case("dark") && output.status.success() {
                    return Self::Light;
                }
                if !output.status.success() {
                    // Command fails when in light mode (key doesn't exist)
                    return Self::Light;
                }
            }
        }

        // 3. Check COLORFGBG env var (format: "fg;bg" where bg > 8 typically means light)
        if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
            if let Some(bg) = colorfgbg.split(';').last() {
                if let Ok(bg_num) = bg.parse::<u8>() {
                    // Background colors 0-8 are typically dark, 9+ are light
                    if bg_num > 8 || bg_num == 7 {
                        return Self::Light;
                    }
                }
            }
        }

        // Default to dark
        Self::Dark
    }
}

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

/// Color palette - adapts to theme
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
    // Logo/branding colors
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
    pub fn dark() -> Self {
        Self {
            added: Color::Rgb(166, 227, 161),      // Green
            removed: Color::Rgb(243, 139, 168),    // Red
            added_bg: Color::Rgb(30, 50, 40),      // Dark green tint
            removed_bg: Color::Rgb(50, 30, 35),    // Dark red tint
            modified: Color::Rgb(250, 179, 135),   // Peach
            renamed: Color::Rgb(203, 166, 247),    // Mauve
            header: Color::Rgb(137, 180, 250),     // Blue
            muted: Color::Rgb(108, 112, 134),      // Overlay0
            text: Color::Rgb(205, 214, 244),       // Text
            border: Color::Rgb(69, 71, 90),        // Surface1
            border_focused: Color::Rgb(137, 180, 250), // Blue
            status_bar: Color::Rgb(49, 50, 68),    // Surface0
            status_bar_text: Color::Rgb(205, 214, 244), // Text
            comment: Color::Rgb(249, 226, 175),    // Yellow
            comment_bg: Color::Rgb(45, 40, 30),    // Warm dark
            logo_primary: Color::Rgb(150, 255, 170),   // Bright green
            logo_highlight: Color::Rgb(255, 80, 80),   // Bright red
        }
    }

    /// Light theme (Catppuccin Latte inspired)
    pub fn light() -> Self {
        Self {
            added: Color::Rgb(64, 160, 43),        // Green (darker for light bg)
            removed: Color::Rgb(210, 15, 57),      // Red
            added_bg: Color::Rgb(220, 245, 220),   // Light green tint
            removed_bg: Color::Rgb(255, 228, 232), // Light red tint
            modified: Color::Rgb(223, 142, 29),    // Orange
            renamed: Color::Rgb(136, 57, 239),     // Purple
            header: Color::Rgb(30, 102, 245),      // Blue
            muted: Color::Rgb(140, 143, 161),      // Gray
            text: Color::Rgb(76, 79, 105),         // Dark text
            border: Color::Rgb(188, 192, 204),     // Light border
            border_focused: Color::Rgb(30, 102, 245), // Blue
            status_bar: Color::Rgb(230, 233, 239), // Light surface
            status_bar_text: Color::Rgb(76, 79, 105), // Dark text
            comment: Color::Rgb(223, 142, 29),     // Orange
            comment_bg: Color::Rgb(255, 248, 230), // Warm light
            logo_primary: Color::Rgb(34, 139, 34),     // Forest green
            logo_highlight: Color::Rgb(200, 30, 30),   // Dark red
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Self::for_theme(ThemeMode::detect())
    }
}

impl Colors {
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
        Style::default()
            .bg(self.status_bar)
            .fg(self.status_bar_text)
    }
}

pub struct Timing {
    pub pr_poll_interval: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            pr_poll_interval: Duration::from_secs(120), // 2 minutes
        }
    }
}
