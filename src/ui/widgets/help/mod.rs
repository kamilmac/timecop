use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::config::Colors;
use crate::event::KeyInput;

/// Help modal state
#[derive(Debug, Default)]
pub struct HelpModalState {
    pub scroll: usize,
    content_height: usize,
}

impl HelpModalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_key(&mut self, key: &KeyEvent) {
        if KeyInput::is_down(key) {
            self.scroll_down(1);
        } else if KeyInput::is_up(key) {
            self.scroll_up(1);
        } else if KeyInput::is_fast_down(key) {
            self.scroll_down(5);
        } else if KeyInput::is_fast_up(key) {
            self.scroll_up(5);
        } else if KeyInput::is_top(key) {
            self.scroll = 0;
        } else if KeyInput::is_bottom(key) {
            self.scroll = self.content_height.saturating_sub(1);
        }
    }

    fn scroll_down(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_add(n);
    }

    fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn set_content_height(&mut self, height: usize) {
        self.content_height = height;
    }
}

/// Help modal widget
pub struct HelpModal<'a> {
    colors: &'a Colors,
}

impl<'a> HelpModal<'a> {
    pub fn new(colors: &'a Colors) -> Self {
        Self { colors }
    }

    fn build_help_text(&self) -> Vec<Line<'a>> {
        vec![
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
                "  T─I─M─E─C─O─P─○─○─○─ ... ─○─○─○─[all]─[wip]",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "                -16          -1   all   wip",
                self.colors.style_muted(),
            )),
            Line::from(Span::styled(
                "  ← older                           newer →",
                self.colors.style_muted(),
            )),
            Line::from(""),
            Line::from(Span::styled("Navigation", self.colors.style_header())),
            format_binding("j/k", "Move up/down", self.colors),
            format_binding("J/K", "Move fast (5 lines)", self.colors),
            format_binding("g/G", "Jump to top/bottom", self.colors),
            format_binding("h/l", "Collapse/expand folders", self.colors),
            format_binding("Tab", "Cycle panes (Files → Preview → PRs)", self.colors),
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
                "Press ? or Esc to close  |  j/k to scroll",
                self.colors.style_muted(),
            )),
        ]
    }
}

impl<'a> StatefulWidget for HelpModal<'a> {
    type State = HelpModalState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
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

        let help_text = self.build_help_text();
        let content_height = help_text.len();
        state.set_content_height(content_height);

        // Clamp scroll to valid range
        let max_scroll = content_height.saturating_sub(inner.height as usize);
        if state.scroll > max_scroll {
            state.scroll = max_scroll;
        }

        let paragraph = Paragraph::new(help_text)
            .wrap(Wrap { trim: false })
            .scroll((state.scroll as u16, 0));

        paragraph.render(inner, buf);
    }
}

fn format_binding<'a>(key: &'a str, desc: &'a str, colors: &'a Colors) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{:>12}", key), colors.style_header()),
        Span::raw("  "),
        Span::styled(desc, ratatui::style::Style::reset().fg(colors.text)),
    ])
}
