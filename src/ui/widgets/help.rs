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
            Line::from(""),
            format_binding("j/k", "Move up/down", self.colors),
            format_binding("J/K", "Fast move (5 lines)", self.colors),
            format_binding("h/l", "Collapse/expand folder", self.colors),
            format_binding("Tab", "Cycle focus clockwise", self.colors),
            format_binding("Ctrl+d/u", "Scroll half page", self.colors),
            format_binding("g/G", "Go to top/bottom", self.colors),
            Line::from(""),
            Line::from(Span::styled("Modes", self.colors.style_header())),
            Line::from(""),
            format_binding("m", "Cycle through modes", self.colors),
            format_binding("1", "changed:working", self.colors),
            format_binding("2", "changed:branch", self.colors),
            format_binding("3", "browse (all files)", self.colors),
            format_binding("4", "docs (markdown)", self.colors),
            Line::from(""),
            Line::from(Span::styled("Actions", self.colors.style_header())),
            Line::from(""),
            format_binding("y", "Copy path to clipboard", self.colors),
            format_binding("o", "Open in $EDITOR", self.colors),
            format_binding("r", "Refresh", self.colors),
            format_binding("?", "Toggle this help", self.colors),
            format_binding("q", "Quit", self.colors),
            Line::from(""),
            Line::from(Span::styled(
                "Press ? or Esc to close",
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
