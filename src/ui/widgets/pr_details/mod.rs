//! PR Details view widget
//!
//! Displays detailed information about a pull request including
//! title, description, reviews, comments, and file comments.

use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::config::Colors;
use crate::event::KeyInput;
use crate::github::PrInfo;

use super::{Action, ScrollState};

/// A parsed line ready for display
#[derive(Debug, Clone)]
struct DisplayLine {
    text: String,
    line_type: LineType,
}

/// Type of line for styling purposes
#[derive(Debug, Clone, Copy, PartialEq)]
enum LineType {
    Header,
    Info,
    Context,
    Added,
    Removed,
    Comment,
}

/// PR details view state
#[derive(Debug, Default)]
pub struct PrDetailsViewState {
    pub pr: Option<PrInfo>,
    pub loading_message: Option<String>,
    lines: Vec<DisplayLine>,
    pub scroll: ScrollState,
}

impl PrDetailsViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_pr(&mut self, pr: Option<PrInfo>) {
        self.pr = pr;
        self.loading_message = None;
        self.scroll = ScrollState::new();
        self.rebuild_lines();
    }

    pub fn set_loading(&mut self, message: String) {
        self.loading_message = Some(message);
        self.pr = None;
        self.lines.clear();
        self.scroll.set_len(0);
    }

    pub fn clear(&mut self) {
        self.pr = None;
        self.loading_message = None;
        self.lines.clear();
        self.scroll = ScrollState::new();
    }

    fn rebuild_lines(&mut self) {
        self.lines = match &self.pr {
            Some(pr) => parse_pr_details(pr),
            None => vec![],
        };
        self.scroll.set_len(self.lines.len());
    }

    pub fn title(&self) -> String {
        match &self.pr {
            Some(pr) => format!("PR #{} {}", pr.number, pr.title),
            None => "PR Details".to_string(),
        }
    }

    /// Handle key input, return action for App to dispatch
    pub fn handle_key(&mut self, key: &KeyEvent) -> Action {
        if KeyInput::is_down(key) {
            self.scroll.move_down();
            Action::None
        } else if KeyInput::is_up(key) {
            self.scroll.move_up();
            Action::None
        } else if KeyInput::is_fast_down(key) {
            self.scroll.move_down_n(5);
            Action::None
        } else if KeyInput::is_fast_up(key) {
            self.scroll.move_up_n(5);
            Action::None
        } else if KeyInput::is_page_down(key) {
            self.scroll.move_down_n(20);
            Action::None
        } else if KeyInput::is_page_up(key) {
            self.scroll.move_up_n(20);
            Action::None
        } else if KeyInput::is_top(key) {
            self.scroll.go_top();
            Action::None
        } else if KeyInput::is_bottom(key) {
            self.scroll.go_bottom();
            Action::None
        } else {
            Action::Ignored
        }
    }
}

/// PR details view widget
pub struct PrDetailsView<'a> {
    colors: &'a Colors,
    focused: bool,
}

impl<'a> PrDetailsView<'a> {
    pub fn new(colors: &'a Colors) -> Self {
        Self {
            colors,
            focused: false,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }
}

impl<'a> StatefulWidget for PrDetailsView<'a> {
    type State = PrDetailsViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = self.colors.border_style(self.focused);

        let scroll_info = state.scroll.scroll_percent(area.height.saturating_sub(2) as usize);
        let title = if scroll_info.is_empty() {
            state.title()
        } else {
            format!("{} ─── {}", state.title(), scroll_info)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Show loading message
        if let Some(msg) = &state.loading_message {
            let line = Line::from(Span::styled(msg, self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        // Show empty state
        if state.lines.is_empty() {
            let msg = "Select a PR to view details";
            let line = Line::from(Span::styled(msg, self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        state.scroll.ensure_visible(inner.height as usize);

        let visible_lines: Vec<_> = state
            .lines
            .iter()
            .enumerate()
            .skip(state.scroll.offset)
            .take(inner.height as usize)
            .collect();

        for (i, (idx, line)) in visible_lines.into_iter().enumerate() {
            let y = inner.y + i as u16;
            let is_cursor = self.focused && idx == state.scroll.cursor;

            let rendered = render_line(line, is_cursor, self.colors);
            buf.set_line(inner.x, y, &rendered, inner.width);
        }
    }
}

fn render_line(line: &DisplayLine, cursor: bool, colors: &Colors) -> Line<'static> {
    let (style, prefix) = match line.line_type {
        LineType::Header => (colors.style_header(), ""),
        LineType::Info => (colors.style_muted(), ""),
        LineType::Context => (Style::reset().fg(colors.text), ""),
        LineType::Added => (colors.style_added(), ""),
        LineType::Removed => (colors.style_removed(), ""),
        LineType::Comment => {
            let style = Style::default()
                .fg(colors.comment)
                .bg(colors.comment_bg);
            (style, "│ ")
        }
    };

    let content_style = if cursor {
        style.add_modifier(ratatui::style::Modifier::REVERSED)
    } else {
        style
    };

    let display_text = format!("{}{}", prefix, line.text);
    Line::from(Span::styled(display_text, content_style))
}

/// Create a display line
fn make_line(text: String, line_type: LineType) -> DisplayLine {
    DisplayLine { text, line_type }
}

/// Parse PR details into display lines
fn parse_pr_details(pr: &PrInfo) -> Vec<DisplayLine> {
    let mut lines = vec![];

    // PR header
    lines.push(make_line(format!("PR #{}", pr.number), LineType::Header));
    lines.push(make_line("─".repeat(40), LineType::Info));
    lines.push(make_line(pr.title.clone(), LineType::Info));
    lines.push(make_line(String::new(), LineType::Context));
    lines.push(make_line(format!("State:  {}", pr.state), LineType::Context));
    lines.push(make_line(format!("Author: @{}", pr.author), LineType::Context));
    lines.push(make_line(format!("URL:    {}", pr.url), LineType::Context));

    // Description
    if !pr.body.is_empty() {
        lines.push(make_line(String::new(), LineType::Context));
        lines.push(make_line("Description".to_string(), LineType::Header));
        lines.push(make_line("─".repeat(40), LineType::Info));
        for line in pr.body.lines() {
            lines.push(make_line(format!("  {}", line), LineType::Context));
        }
    }

    // Reviews
    if !pr.reviews.is_empty() {
        lines.push(make_line(String::new(), LineType::Context));
        lines.push(make_line("Reviews".to_string(), LineType::Header));
        lines.push(make_line("─".repeat(40), LineType::Info));
        for review in &pr.reviews {
            let (icon, line_type) = match review.state.as_str() {
                "APPROVED" => ("✓", LineType::Added),
                "CHANGES_REQUESTED" => ("✗", LineType::Removed),
                _ => ("○", LineType::Context),
            };
            lines.push(make_line(
                format!("  {} {} - {}", icon, review.author, review.state),
                line_type,
            ));
            if !review.body.is_empty() {
                for line in review.body.lines() {
                    lines.push(make_line(format!("    {}", line), LineType::Context));
                }
            }
        }
    }

    // General comments
    if !pr.comments.is_empty() {
        lines.push(make_line(String::new(), LineType::Context));
        lines.push(make_line("Comments".to_string(), LineType::Header));
        lines.push(make_line("─".repeat(40), LineType::Info));
        for comment in &pr.comments {
            lines.push(make_line(format!("  💬 {}", comment.author), LineType::Comment));
            for line in comment.body.lines() {
                lines.push(make_line(format!("    {}", line), LineType::Context));
            }
            lines.push(make_line(String::new(), LineType::Context));
        }
    }

    // File comments (grouped by file)
    if !pr.file_comments.is_empty() {
        lines.push(make_line(String::new(), LineType::Context));
        lines.push(make_line("File Comments".to_string(), LineType::Header));
        lines.push(make_line("─".repeat(40), LineType::Info));

        for (path, comments) in &pr.file_comments {
            lines.push(make_line(format!("  {}", path), LineType::Info));
            for comment in comments {
                let line_info = comment
                    .line
                    .map(|l| format!(":{}", l))
                    .unwrap_or_default();
                lines.push(make_line(
                    format!("    💬 @{}{}", comment.author, line_info),
                    LineType::Comment,
                ));
                for line in comment.body.lines() {
                    lines.push(make_line(format!("      {}", line), LineType::Context));
                }
            }
            lines.push(make_line(String::new(), LineType::Context));
        }
    }

    lines
}
