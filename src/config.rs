use ratatui::style::{Color, Modifier, Style};
use std::time::Duration;

/// Application configuration
pub struct Config {
    pub colors: Colors,
    pub timing: Timing,
    pub layout: Layout,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colors: Colors::default(),
            timing: Timing::default(),
            layout: Layout::default(),
        }
    }
}

/// Catppuccin Mocha color palette
pub struct Colors {
    pub added: Color,
    pub removed: Color,
    pub added_bg: Color,
    pub removed_bg: Color,
    pub modified: Color,
    pub renamed: Color,
    pub untracked: Color,
    pub header: Color,
    pub muted: Color,
    pub text: Color,
    pub surface: Color,
    pub base: Color,
    pub border: Color,
    pub border_focused: Color,
    pub cursor: Color,
    pub status_bar: Color,
    pub status_bar_text: Color,
    pub comment: Color,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            added: Color::Rgb(166, 227, 161),      // Green
            removed: Color::Rgb(243, 139, 168),    // Red
            added_bg: Color::Rgb(30, 50, 40),      // Dark green tint
            removed_bg: Color::Rgb(50, 30, 35),    // Dark red tint
            modified: Color::Rgb(250, 179, 135),   // Peach
            renamed: Color::Rgb(203, 166, 247),    // Mauve
            untracked: Color::Rgb(108, 112, 134),  // Overlay0
            header: Color::Rgb(137, 180, 250),     // Blue
            muted: Color::Rgb(108, 112, 134),      // Overlay0
            text: Color::Rgb(205, 214, 244),       // Text
            surface: Color::Rgb(49, 50, 68),       // Surface0
            base: Color::Rgb(30, 30, 46),          // Base
            border: Color::Rgb(69, 71, 90),        // Surface1
            border_focused: Color::Rgb(137, 180, 250), // Blue
            cursor: Color::Rgb(245, 224, 220),     // Rosewater
            status_bar: Color::Rgb(49, 50, 68),    // Surface0
            status_bar_text: Color::Rgb(205, 214, 244), // Text
            comment: Color::Rgb(249, 226, 175),   // Yellow for PR comments
        }
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

    pub fn style_cursor(&self) -> Style {
        Style::default().add_modifier(Modifier::REVERSED)
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
    pub file_watcher_debounce: Duration,
    pub pr_poll_interval: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Self {
            file_watcher_debounce: Duration::from_millis(500),
            pr_poll_interval: Duration::from_secs(60),
        }
    }
}

pub struct Layout {
    pub left_ratio: u16,
    pub right_ratio: u16,
    pub breakpoint: u16,
    pub diff_pane_min_width: u16,
    pub diff_line_num_width: usize,
    pub diff_tab_width: usize,
    pub max_commits: usize,
    /// In browse/docs mode, collapse folders at this depth and below (0 = root collapsed, 1 = first level collapsed)
    pub browse_collapse_depth: usize,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            left_ratio: 30,
            right_ratio: 70,
            breakpoint: 80,
            diff_pane_min_width: 40,
            diff_line_num_width: 4,
            diff_tab_width: 4,
            max_commits: 8,
            browse_collapse_depth: 2, // Collapse folders at depth 2+
        }
    }
}
