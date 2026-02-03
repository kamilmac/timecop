use ratatui::style::{Color, Modifier, Style};
use std::io::{IsTerminal, Read, Write};
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

        // 2. Query terminal background color via OSC 11 (most accurate)
        if let Some(theme) = Self::detect_from_terminal() {
            return theme;
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

    /// Query terminal for background color using OSC 11 escape sequence
    fn detect_from_terminal() -> Option<Self> {
        use std::os::fd::{AsRawFd, BorrowedFd};

        // Only works on a real terminal
        let stdin = std::io::stdin();
        if !stdin.is_terminal() {
            return None;
        }

        // Save current terminal settings
        let original_termios = match nix::sys::termios::tcgetattr(&stdin) {
            Ok(t) => t,
            Err(_) => return None,
        };

        // Set terminal to raw mode for reading response
        let mut raw_termios = original_termios.clone();
        nix::sys::termios::cfmakeraw(&mut raw_termios);
        raw_termios.local_flags.insert(nix::sys::termios::LocalFlags::ISIG);

        if nix::sys::termios::tcsetattr(
            &stdin,
            nix::sys::termios::SetArg::TCSANOW,
            &raw_termios,
        ).is_err() {
            return None;
        }

        // Send OSC 11 query: request background color
        // Format: ESC ] 11 ; ? BEL  or  ESC ] 11 ; ? ESC \
        let query = "\x1b]11;?\x1b\\";
        let _ = std::io::stdout().write_all(query.as_bytes());
        let _ = std::io::stdout().flush();

        // Read response with timeout
        let mut response = Vec::new();
        let mut buf = [0u8; 1];
        let deadline = std::time::Instant::now() + Duration::from_millis(100);

        // Set non-blocking read with timeout using poll
        let stdin_fd = stdin.as_raw_fd();
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
        let mut poll_fds = [nix::poll::PollFd::new(
            borrowed_fd,
            nix::poll::PollFlags::POLLIN,
        )];

        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let timeout_ms = remaining.as_millis() as u16;

            if nix::poll::poll(&mut poll_fds, nix::poll::PollTimeout::from(timeout_ms)).unwrap_or(0) > 0 {
                if std::io::stdin().read(&mut buf).unwrap_or(0) == 1 {
                    response.push(buf[0]);
                    // Check for terminator (BEL or ST)
                    if buf[0] == 0x07 || (response.len() >= 2 && response.ends_with(b"\x1b\\")) {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        // Restore terminal settings
        let _ = nix::sys::termios::tcsetattr(
            &stdin,
            nix::sys::termios::SetArg::TCSANOW,
            &original_termios,
        );

        // Parse response: ESC ] 11 ; rgb:RRRR/GGGG/BBBB BEL
        let response_str = String::from_utf8_lossy(&response);
        Self::parse_osc11_response(&response_str)
    }

    /// Parse OSC 11 response and determine theme from background color
    fn parse_osc11_response(response: &str) -> Option<Self> {
        // Response format: \x1b]11;rgb:RRRR/GGGG/BBBB\x07
        // or with 2-digit hex: \x1b]11;rgb:RR/GG/BB\x07
        let rgb_start = response.find("rgb:")?;
        let rgb_part = &response[rgb_start + 4..];

        // Find end (before BEL or ST)
        let rgb_end = rgb_part.find(|c| c == '\x07' || c == '\x1b')
            .unwrap_or(rgb_part.len());
        let rgb_str = &rgb_part[..rgb_end];

        let parts: Vec<&str> = rgb_str.split('/').collect();
        if parts.len() != 3 {
            return None;
        }

        // Parse hex values (can be 2 or 4 digits)
        let r = u16::from_str_radix(parts[0], 16).ok()?;
        let g = u16::from_str_radix(parts[1], 16).ok()?;
        let b = u16::from_str_radix(parts[2], 16).ok()?;

        // Normalize to 0-255 range (4-digit hex = 0-65535)
        let (r, g, b) = if parts[0].len() > 2 {
            ((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8)
        } else {
            (r as u8, g as u8, b as u8)
        };

        // Calculate relative luminance (ITU-R BT.709)
        // L = 0.2126*R + 0.7152*G + 0.0722*B
        let luminance = 0.2126 * (r as f64) + 0.7152 * (g as f64) + 0.0722 * (b as f64);

        // Threshold at ~50% brightness
        if luminance > 128.0 {
            Some(Self::Light)
        } else {
            Some(Self::Dark)
        }
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

    /// Light theme (high contrast for light backgrounds)
    pub fn light() -> Self {
        Self {
            added: Color::Rgb(0, 110, 0),          // Dark green
            removed: Color::Rgb(180, 0, 30),       // Dark red
            added_bg: Color::Rgb(210, 245, 210),   // Light green tint
            removed_bg: Color::Rgb(255, 215, 220), // Light red tint
            modified: Color::Rgb(160, 80, 0),      // Dark orange
            renamed: Color::Rgb(90, 20, 180),      // Dark purple
            header: Color::Rgb(0, 60, 180),        // Dark blue
            muted: Color::Rgb(60, 60, 70),         // Dark gray (not light!)
            text: Color::Rgb(10, 10, 15),          // Almost black
            border: Color::Rgb(150, 155, 170),     // Visible border
            border_focused: Color::Rgb(0, 60, 180), // Dark blue
            status_bar: Color::Rgb(220, 225, 235), // Light surface
            status_bar_text: Color::Rgb(10, 10, 15), // Almost black
            comment: Color::Rgb(160, 80, 0),       // Dark orange
            comment_bg: Color::Rgb(255, 245, 220), // Warm light
            logo_primary: Color::Rgb(0, 90, 30),       // Dark green
            logo_highlight: Color::Rgb(160, 0, 0),     // Dark red
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
