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
}

impl Highlighter {
    pub fn new() -> Self {
        Self::for_theme(ThemeMode::detect())
    }

    pub fn for_theme(mode: ThemeMode) -> Self {
        let theme_name = match mode {
            ThemeMode::Dark => "base16-eighties.dark",
            ThemeMode::Light => "InspiredGitHub",
        };
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: theme_name.to_string(),
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

        for line in content.lines() {
            let ranges = highlighter.highlight_line(line, &self.syntax_set)
                .unwrap_or_default();

            let styled_spans: Vec<(String, Style)> = ranges
                .into_iter()
                .map(|(style, text)| {
                    let ratatui_style = syntect_to_ratatui_style(&style);
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
fn syntect_to_ratatui_style(style: &syntect::highlighting::Style) -> Style {
    let fg = syntect_to_ratatui_color(style.foreground);

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
