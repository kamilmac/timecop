use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style as SynStyle, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl Highlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get("base16-ocean.dark")
            .cloned()
            .unwrap_or_else(|| {
                theme_set
                    .themes
                    .values()
                    .next()
                    .cloned()
                    .expect("at least one theme")
            });
        Self { syntax_set, theme }
    }

    fn syntax_for(&self, path: &str) -> &SyntaxReference {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        self.syntax_set
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    pub fn highlight(&self, path: &str, text: &str) -> Vec<Span<'static>> {
        if text.is_empty() {
            return vec![Span::raw("")];
        }
        let syntax = self.syntax_for(path);
        let mut h = HighlightLines::new(syntax, &self.theme);
        let with_newline = format!("{text}\n");
        let ranges = match h.highlight_line(&with_newline, &self.syntax_set) {
            Ok(r) => r,
            Err(_) => return vec![Span::raw(text.to_string())],
        };
        ranges
            .into_iter()
            .map(|(style, slice)| {
                let cleaned = slice.trim_end_matches('\n').to_string();
                Span::styled(cleaned, convert_style(style))
            })
            .filter(|s| !s.content.is_empty())
            .collect()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

fn convert_style(s: SynStyle) -> Style {
    let mut out = Style::default().fg(Color::Rgb(s.foreground.r, s.foreground.g, s.foreground.b));
    if s.font_style.contains(FontStyle::BOLD) {
        out = out.add_modifier(Modifier::BOLD);
    }
    if s.font_style.contains(FontStyle::ITALIC) {
        out = out.add_modifier(Modifier::ITALIC);
    }
    if s.font_style.contains(FontStyle::UNDERLINE) {
        out = out.add_modifier(Modifier::UNDERLINED);
    }
    out
}
