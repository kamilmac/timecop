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
                "TIMECOP - Time-Travel Code Review",
                self.colors.style_header(),
            ))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        block.render(area, buf);

        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Navigate through commit history. The TIMECOP title is your timeline.",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  Selected position glows red. Use , and . to time-travel.",
                self.colors.style_muted(),
            )),
            Line::from(""),
            Line::from(Span::styled("Timeline", self.colors.style_header())),
            Line::from(Span::styled(
                "  ◆───T───I───M───E───C───O───P───◆",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  │   │   │   │   │   │   │   │   └── all changes",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  │   │   │   │   │   │   │   └────── wip",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  │   │   │   │   │   │   └────────── -1",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  │   │   │   │   │   └────────────── -2",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  └───┴───┴───┴───┴──────────────────  -3 to -7",
                self.colors.style_muted(),
            )),
            Line::from(""),
            Line::from(Span::styled("Navigation", self.colors.style_header())),
            format_binding("j/k", "Move up/down", self.colors),
            format_binding("J/K", "Move fast (5 lines)", self.colors),
            format_binding("g/G", "Jump to top/bottom", self.colors),
            format_binding("h/l", "Collapse/expand folders", self.colors),
            format_binding("Tab", "Cycle panes (Files → PRs → Preview)", self.colors),
            format_binding("Enter", "Open diff / Checkout PR", self.colors),
            format_binding("Esc", "Back to file list", self.colors),
            format_binding(",", "Timeline: older commit", self.colors),
            format_binding(".", "Timeline: newer / full diff", self.colors),
            Line::from(""),
            Line::from(Span::styled("Diff View", self.colors.style_header())),
            format_binding("s", "Toggle split/unified view", self.colors),
            Line::from(""),
            Line::from(Span::styled("Actions", self.colors.style_header())),
            format_binding("o", "Open in $EDITOR (or PR in browser)", self.colors),
            format_binding("y", "Copy path to clipboard", self.colors),
            format_binding("r", "Refresh", self.colors),
            format_binding("q", "Quit", self.colors),
            Line::from(""),
            Line::from(Span::styled("PR Review", self.colors.style_header())),
            format_binding("a", "Approve", self.colors),
            format_binding("x", "Request changes", self.colors),
            format_binding("c", "Comment (PR or line)", self.colors),
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
