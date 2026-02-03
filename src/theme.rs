//! Terminal theme detection
//!
//! Detects whether the terminal is using a light or dark color scheme.
//! Uses multiple detection methods in order of reliability:
//! 1. TIMECOP_THEME environment variable (explicit override)
//! 2. OSC 11 terminal query (most accurate, queries actual background color)
//! 3. COLORFGBG environment variable (set by some terminals)
//! 4. Terminal-specific hints (iTerm2, Kitty, VS Code, Terminal.app)

use std::io::{IsTerminal, Read, Write};
use std::time::Duration;

/// Theme mode for the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    /// Detect theme mode from terminal environment
    pub fn detect() -> Self {
        // 1. Explicit override via environment variable
        if let Some(theme) = Self::from_env() {
            return theme;
        }

        // 2. Query terminal background color directly (most accurate)
        if let Some(theme) = Self::query_terminal_background() {
            return theme;
        }

        // 3. Check COLORFGBG (set by xterm, rxvt, and others)
        if let Some(theme) = Self::from_colorfgbg() {
            return theme;
        }

        // 4. Terminal-specific hints
        if let Some(theme) = Self::from_terminal_hints() {
            return theme;
        }

        // Default to dark (most common terminal setup)
        Self::Dark
    }

    /// Check TIMECOP_THEME environment variable
    fn from_env() -> Option<Self> {
        let theme = std::env::var("TIMECOP_THEME").ok()?;
        match theme.to_lowercase().as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }

    /// Check COLORFGBG environment variable (format: "fg;bg")
    fn from_colorfgbg() -> Option<Self> {
        let colorfgbg = std::env::var("COLORFGBG").ok()?;
        let bg = colorfgbg.split(';').last()?;
        let bg_num: u8 = bg.parse().ok()?;
        // ANSI colors 0-6,8 are typically dark, 7 and 9+ are light
        if bg_num > 8 || bg_num == 7 {
            Some(Self::Light)
        } else {
            None
        }
    }

    /// Check terminal-specific environment hints
    fn from_terminal_hints() -> Option<Self> {
        // iTerm2
        if let Ok(profile) = std::env::var("ITERM_PROFILE") {
            let lower = profile.to_lowercase();
            if lower.contains("light") {
                return Some(Self::Light);
            }
            if lower.contains("dark") {
                return Some(Self::Dark);
            }
        }

        // Kitty
        if let Ok(theme) = std::env::var("KITTY_THEME") {
            if theme.to_lowercase().contains("light") {
                return Some(Self::Light);
            }
        }

        // VS Code integrated terminal
        if let Ok(theme) = std::env::var("VSCODE_TERMINAL_THEME") {
            if theme.to_lowercase().contains("light") {
                return Some(Self::Light);
            }
        }

        // macOS Terminal.app
        #[cfg(target_os = "macos")]
        {
            if let Some(theme) = Self::from_macos_terminal() {
                return Some(theme);
            }
        }

        None
    }

    /// Detect theme from macOS Terminal.app profile
    #[cfg(target_os = "macos")]
    fn from_macos_terminal() -> Option<Self> {
        if std::env::var("TERM_PROGRAM").ok()? != "Apple_Terminal" {
            return None;
        }

        let output = std::process::Command::new("defaults")
            .args(["read", "com.apple.Terminal", "Default Window Settings"])
            .output()
            .ok()?;

        let profile = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase();

        // Known light profiles in Terminal.app
        const LIGHT_PROFILES: &[&str] = &["basic", "novel", "ocean", "grass", "silver aerogel"];
        if LIGHT_PROFILES.contains(&profile.as_str()) || profile.contains("light") {
            Some(Self::Light)
        } else {
            None
        }
    }

    /// Query terminal background color using OSC 11 escape sequence
    #[cfg(unix)]
    fn query_terminal_background() -> Option<Self> {
        let stdin = std::io::stdin();
        if !stdin.is_terminal() {
            return None;
        }

        // Save terminal state
        let original = nix::sys::termios::tcgetattr(&stdin).ok()?;

        // Enter raw mode to read response
        let mut raw = original.clone();
        nix::sys::termios::cfmakeraw(&mut raw);
        raw.local_flags.insert(nix::sys::termios::LocalFlags::ISIG);
        nix::sys::termios::tcsetattr(&stdin, nix::sys::termios::SetArg::TCSANOW, &raw).ok()?;

        // Send OSC 11 query (BEL terminator for wider compatibility)
        let _ = std::io::stdout().write_all(b"\x1b]11;?\x07");
        let _ = std::io::stdout().flush();

        // Read response with timeout
        let response = Self::read_osc_response(&stdin, Duration::from_millis(300));

        // Restore terminal state
        let _ = nix::sys::termios::tcsetattr(&stdin, nix::sys::termios::SetArg::TCSANOW, &original);

        Self::parse_osc11_response(&response)
    }

    /// Windows: OSC 11 query not supported, skip this detection method
    #[cfg(not(unix))]
    fn query_terminal_background() -> Option<Self> {
        None
    }

    /// Read OSC response from terminal with timeout (Unix only)
    #[cfg(unix)]
    fn read_osc_response(stdin: &std::io::Stdin, timeout: Duration) -> String {
        use std::os::fd::{AsRawFd, BorrowedFd};

        let mut response = Vec::new();
        let mut buf = [0u8; 1];
        let deadline = std::time::Instant::now() + timeout;

        let stdin_fd = stdin.as_raw_fd();
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(stdin_fd) };
        let mut poll_fds = [nix::poll::PollFd::new(borrowed_fd, nix::poll::PollFlags::POLLIN)];

        while std::time::Instant::now() < deadline {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let timeout_ms = remaining.as_millis().min(u16::MAX as u128) as u16;

            if nix::poll::poll(&mut poll_fds, nix::poll::PollTimeout::from(timeout_ms)).unwrap_or(0) > 0 {
                if std::io::stdin().read(&mut buf).unwrap_or(0) == 1 {
                    response.push(buf[0]);
                    // Check for terminators: BEL (\x07) or ST (\x1b\\)
                    if buf[0] == 0x07 || response.ends_with(b"\x1b\\") {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        String::from_utf8_lossy(&response).into_owned()
    }

    /// Parse OSC 11 response to determine theme
    /// Response format: \x1b]11;rgb:RRRR/GGGG/BBBB\x07
    fn parse_osc11_response(response: &str) -> Option<Self> {
        let rgb_start = response.find("rgb:")?;
        let rgb_part = &response[rgb_start + 4..];
        let rgb_end = rgb_part.find(|c| c == '\x07' || c == '\x1b').unwrap_or(rgb_part.len());
        let rgb_str = &rgb_part[..rgb_end];

        let parts: Vec<&str> = rgb_str.split('/').collect();
        if parts.len() != 3 {
            return None;
        }

        // Parse hex values (can be 2 or 4 digits per component)
        let r = u16::from_str_radix(parts[0], 16).ok()?;
        let g = u16::from_str_radix(parts[1], 16).ok()?;
        let b = u16::from_str_radix(parts[2], 16).ok()?;

        // Normalize to 0-255 range
        let (r, g, b) = if parts[0].len() > 2 {
            ((r >> 8) as u8, (g >> 8) as u8, (b >> 8) as u8)
        } else {
            (r as u8, g as u8, b as u8)
        };

        // Calculate luminance (ITU-R BT.709)
        let luminance = 0.2126 * (r as f64) + 0.7152 * (g as f64) + 0.0722 * (b as f64);

        if luminance > 128.0 {
            Some(Self::Light)
        } else {
            Some(Self::Dark)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_osc11_response_4digit() {
        // White background (ffff/ffff/ffff)
        let response = "\x1b]11;rgb:ffff/ffff/ffff\x07";
        assert_eq!(ThemeMode::parse_osc11_response(response), Some(ThemeMode::Light));

        // Black background (0000/0000/0000)
        let response = "\x1b]11;rgb:0000/0000/0000\x07";
        assert_eq!(ThemeMode::parse_osc11_response(response), Some(ThemeMode::Dark));
    }

    #[test]
    fn test_parse_osc11_response_2digit() {
        let response = "\x1b]11;rgb:ff/ff/ff\x07";
        assert_eq!(ThemeMode::parse_osc11_response(response), Some(ThemeMode::Light));

        let response = "\x1b]11;rgb:00/00/00\x07";
        assert_eq!(ThemeMode::parse_osc11_response(response), Some(ThemeMode::Dark));
    }

    #[test]
    fn test_parse_osc11_invalid() {
        assert_eq!(ThemeMode::parse_osc11_response("invalid"), None);
        assert_eq!(ThemeMode::parse_osc11_response(""), None);
    }
}
