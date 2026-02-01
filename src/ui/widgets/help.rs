use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::config::Colors;

/// Help modal widget
pub struct HelpModal<'a> {
    colors: &'a Colors,
}

impl<'a> HelpModal<'a> {
    pub fn new(colors: &'a Colors) -> Self {
        Self { colors }
    }
}

impl<'a> Widget for HelpModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.colors.style_border_focused())
            .title(Span::styled(
                "Kimchi - AI-Native Code Review",
                self.colors.style_header(),
            ))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        block.render(area, buf);

        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled("Navigation", self.colors.style_header())),
            format_binding("j/k", "Up/down", self.colors),
            format_binding("J/K", "Fast (5 lines)", self.colors),
            format_binding("h/l", "Collapse/expand", self.colors),
            format_binding("Tab", "Next window", self.colors),
            format_binding("Ctrl+d/u", "Page down/up", self.colors),
            format_binding("g/G", "Top/bottom", self.colors),
            Line::from(""),
            Line::from(Span::styled("Modes", self.colors.style_header())),
            format_binding("1", "Working  - uncommitted changes (git diff)", self.colors),
            format_binding("2", "Branch   - vs base branch (git diff main)", self.colors),
            format_binding("3", "Browse   - all tracked files", self.colors),
            format_binding("4", "Docs     - markdown files only", self.colors),
            format_binding("m", "Cycle modes", self.colors),
            Line::from(""),
            Line::from(Span::styled("Actions", self.colors.style_header())),
            format_binding("y", "Yank path (with line number in diff)", self.colors),
            format_binding("o", "Open in $EDITOR at line", self.colors),
            format_binding("r", "Refresh git data", self.colors),
            format_binding("?", "Toggle help", self.colors),
            format_binding("q", "Quit", self.colors),
            Line::from(""),
            Line::from(Span::styled(
                "? or Esc to close",
                self.colors.style_muted(),
            )),
        ];

        let paragraph = Paragraph::new(help_text).wrap(Wrap { trim: false });

        paragraph.render(inner, buf);
    }
}

fn format_binding<'a>(key: &'a str, desc: &'a str, colors: &'a Colors) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:>12}", key), colors.style_header()),
        Span::raw("  "),
        Span::styled(desc, ratatui::style::Style::default().fg(colors.text)),
    ])
}
