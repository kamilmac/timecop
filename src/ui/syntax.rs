use ratatui::style::{Color, Modifier, Style};
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::easy::HighlightLines;

use crate::config::ThemeMode;

/// Syntax highlighter using syntect
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
    theme_mode: ThemeMode,
}

impl Highlighter {
    pub fn new() -> Self {
        Self::for_theme(ThemeMode::detect())
    }

    pub fn for_theme(mode: ThemeMode) -> Self {
        // Use dark theme for both - we'll adjust colors manually for light mode
        let theme_name = "base16-eighties.dark";
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: theme_name.to_string(),
            theme_mode: mode,
        }
    }

    /// Highlight a file's content, returning styled lines
    pub fn highlight_file(&self, content: &str, path: &str) -> Vec<Vec<(String, Style)>> {
        let extension = path.rsplit('.').next().unwrap_or("");

        // Map extensions to syntax names for better coverage
        let mapped_ext = match extension {
            "ts" | "tsx" | "mts" | "cts" => "typescript",
            "jsx" | "mjs" | "cjs" => "js",
            "yml" => "yaml",
            "md" => "markdown",
            "dockerfile" => "Dockerfile",
            ext => ext,
        };

        // Try mapped extension first, then original, then first line detection
        // For TypeScript, fall back to JavaScript if no TS syntax available

        let syntax = self.syntax_set
            .find_syntax_by_extension(mapped_ext)
            .or_else(|| self.syntax_set.find_syntax_by_extension(extension))
            // TypeScript fallback to JavaScript
            .or_else(|| {
                if matches!(extension, "ts" | "tsx" | "mts" | "cts") {
                    self.syntax_set.find_syntax_by_extension("js")
                } else {
                    None
                }
            })
            .or_else(|| self.syntax_set.find_syntax_by_first_line(content.lines().next().unwrap_or("")))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self.theme_set.themes.get(&self.theme_name)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = Vec::new();
        let is_light = self.theme_mode == ThemeMode::Light;

        for line in content.lines() {
            let ranges = highlighter.highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let styled_spans: Vec<(String, Style)> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let ratatui_style = syntect_to_ratatui_style(&style, is_light);
                    (text.to_string(), ratatui_style)
                })
                .collect();

            result.push(styled_spans);
        }

        result
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert syntect style to ratatui style (foreground only, no background)
fn syntect_to_ratatui_style(style: &syntect::highlighting::Style, is_light: bool) -> Style {
    let fg = if is_light {
        // For light mode: darken all colors significantly for readability
        darken_for_light_mode(style.foreground)
    } else {
        syntect_to_ratatui_color(style.foreground)
    };

    // Use reset() to ensure no background color bleeds through
    let mut ratatui_style = Style::reset().fg(fg);

    if style.font_style.contains(FontStyle::BOLD) {
        ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
    }
    if style.font_style.contains(FontStyle::ITALIC) {
        ratatui_style = ratatui_style.add_modifier(Modifier::ITALIC);
    }
    if style.font_style.contains(FontStyle::UNDERLINE) {
        ratatui_style = ratatui_style.add_modifier(Modifier::UNDERLINED);
    }

    ratatui_style
}

/// Convert syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Darken colors for light mode - convert bright colors to dark equivalents
fn darken_for_light_mode(color: syntect::highlighting::Color) -> Color {
    // Convert to HSL-like adjustment: reduce lightness significantly
    let r = color.r as f32 / 255.0;
    let g = color.g as f32 / 255.0;
    let b = color.b as f32 / 255.0;

    // Calculate luminance
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;

    // If the color is bright (designed for dark bg), darken it substantially
    let (new_r, new_g, new_b) = if lum > 0.5 {
        // Darken bright colors - multiply by factor to reduce brightness
        let factor = 0.35; // Make quite dark
        (
            (r * factor * 255.0) as u8,
            (g * factor * 255.0) as u8,
            (b * factor * 255.0) as u8,
        )
    } else {
        // Already dark, keep as is or slightly adjust
        (color.r, color.g, color.b)
    };

    Color::Rgb(new_r, new_g, new_b)
}
