use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::config::Colors;
use crate::git::Commit;
use crate::github::PrInfo;

/// What to show in the diff view
#[derive(Debug, Clone)]
pub enum PreviewContent {
    Empty,
    FileDiff {
        path: String,
        content: String,
    },
    FolderDiff {
        path: String,
        content: String,
    },
    FileContent {
        path: String,
        content: String,
    },
    CommitSummary {
        commit: Commit,
        pr: Option<PrInfo>,
    },
}

impl Default for PreviewContent {
    fn default() -> Self {
        Self::Empty
    }
}

/// Diff view widget state
#[derive(Debug, Default)]
pub struct DiffViewState {
    pub content: PreviewContent,
    pub lines: Vec<DiffLine>,
    pub cursor: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub left_text: Option<String>,
    pub right_text: Option<String>,
    pub left_num: Option<usize>,
    pub right_num: Option<usize>,
    pub line_type: LineType,
    pub is_header: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineType {
    Context,
    Added,
    Removed,
    Header,
    Info,
}

impl DiffViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_content(&mut self, content: PreviewContent) {
        self.content = content;
        self.cursor = 0;
        self.offset = 0;
        self.parse_content();
    }

    fn parse_content(&mut self) {
        self.lines = match &self.content {
            PreviewContent::Empty => vec![],
            PreviewContent::FileDiff { content, .. } | PreviewContent::FolderDiff { content, .. } => {
                if is_binary(content) {
                    vec![DiffLine {
                        left_text: Some("Binary file".to_string()),
                        right_text: None,
                        left_num: None,
                        right_num: None,
                        line_type: LineType::Info,
                        is_header: false,
                    }]
                } else {
                    parse_diff(content)
                }
            }
            PreviewContent::FileContent { content, .. } => {
                if is_binary(content) {
                    vec![DiffLine {
                        left_text: Some("Binary file".to_string()),
                        right_text: None,
                        left_num: None,
                        right_num: None,
                        line_type: LineType::Info,
                        is_header: false,
                    }]
                } else {
                    parse_file_content(content)
                }
            }
            PreviewContent::CommitSummary { commit, pr } => {
                parse_commit_summary(commit, pr.as_ref())
            }
        };
    }

    pub fn title(&self) -> String {
        match &self.content {
            PreviewContent::Empty => "Preview".to_string(),
            PreviewContent::FileDiff { path, .. } => path.clone(),
            PreviewContent::FolderDiff { path, .. } => format!("{}/", path),
            PreviewContent::FileContent { path, .. } => path.clone(),
            PreviewContent::CommitSummary { commit, .. } => {
                format!("{} {}", commit.short_hash, commit.subject)
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor < self.lines.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down_n(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.lines.len().saturating_sub(1));
    }

    pub fn move_up_n(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    pub fn go_top(&mut self) {
        self.cursor = 0;
        self.offset = 0;
    }

    pub fn go_bottom(&mut self) {
        self.cursor = self.lines.len().saturating_sub(1);
    }

    pub fn page_down(&mut self, amount: usize) {
        self.move_down_n(amount);
    }

    pub fn page_up(&mut self, amount: usize) {
        self.move_up_n(amount);
    }

    pub fn get_current_line_number(&self) -> Option<usize> {
        self.lines.get(self.cursor).and_then(|line| line.right_num.or(line.left_num))
    }

    pub fn ensure_visible(&mut self, height: usize) {
        let visible_height = height.saturating_sub(1);
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + visible_height {
            self.offset = self.cursor.saturating_sub(visible_height) + 1;
        }
    }

    pub fn scroll_percent(&self, height: usize) -> String {
        if self.lines.is_empty() || self.lines.len() <= height.saturating_sub(2) {
            return String::new();
        }
        let percent = (self.offset * 100) / self.lines.len().saturating_sub(height.saturating_sub(2)).max(1);
        format!("{}%", percent.min(100))
    }

    pub fn yank_content(&self) -> Option<String> {
        match &self.content {
            PreviewContent::FileDiff { content, .. } => Some(content.clone()),
            PreviewContent::FolderDiff { content, .. } => Some(content.clone()),
            PreviewContent::FileContent { content, .. } => Some(content.clone()),
            _ => None,
        }
    }
}

fn is_binary(content: &str) -> bool {
    let check_len = content.len().min(8192);
    content[..check_len].contains('\0')
}

fn parse_diff(content: &str) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut left_num = 1usize;
    let mut right_num = 1usize;

    for line in content.lines() {
        if line.starts_with("@@") {
            if let Some((l, r)) = parse_hunk_header(line) {
                left_num = l;
                right_num = r;
            }
            lines.push(DiffLine {
                left_text: Some(line.to_string()),
                right_text: None,
                left_num: None,
                right_num: None,
                line_type: LineType::Header,
                is_header: true,
            });
        } else if line.starts_with("diff --git") || line.starts_with("index ")
            || line.starts_with("---") || line.starts_with("+++")
            || line.starts_with("new file") || line.starts_with("deleted file")
        {
            lines.push(DiffLine {
                left_text: Some(line.to_string()),
                right_text: None,
                left_num: None,
                right_num: None,
                line_type: LineType::Header,
                is_header: true,
            });
        } else if line.starts_with('+') {
            lines.push(DiffLine {
                left_text: None,
                right_text: Some(line[1..].to_string()),
                left_num: None,
                right_num: Some(right_num),
                line_type: LineType::Added,
                is_header: false,
            });
            right_num += 1;
        } else if line.starts_with('-') {
            lines.push(DiffLine {
                left_text: Some(line[1..].to_string()),
                right_text: None,
                left_num: Some(left_num),
                right_num: None,
                line_type: LineType::Removed,
                is_header: false,
            });
            left_num += 1;
        } else if line.starts_with(' ') {
            lines.push(DiffLine {
                left_text: Some(line[1..].to_string()),
                right_text: Some(line[1..].to_string()),
                left_num: Some(left_num),
                right_num: Some(right_num),
                line_type: LineType::Context,
                is_header: false,
            });
            left_num += 1;
            right_num += 1;
        } else {
            lines.push(DiffLine {
                left_text: Some(line.to_string()),
                right_text: None,
                left_num: None,
                right_num: None,
                line_type: LineType::Context,
                is_header: true,
            });
        }
    }

    lines
}

fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    // Parse @@ -start,count +start,count @@
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let left_start = parts.get(1)?
        .trim_start_matches('-')
        .split(',')
        .next()?
        .parse()
        .ok()?;

    let right_start = parts.get(2)?
        .trim_start_matches('+')
        .split(',')
        .next()?
        .parse()
        .ok()?;

    Some((left_start, right_start))
}

fn parse_file_content(content: &str) -> Vec<DiffLine> {
    content
        .lines()
        .enumerate()
        .map(|(i, line)| DiffLine {
            left_text: Some(line.to_string()),
            right_text: Some(line.to_string()),
            left_num: Some(i + 1),
            right_num: Some(i + 1),
            line_type: LineType::Context,
            is_header: false,
        })
        .collect()
}

fn make_header_line(text: String, line_type: LineType) -> DiffLine {
    DiffLine {
        left_text: Some(text),
        right_text: None,
        left_num: None,
        right_num: None,
        line_type,
        is_header: true,
    }
}

fn parse_commit_summary(commit: &Commit, pr: Option<&PrInfo>) -> Vec<DiffLine> {
    let mut lines = vec![];

    // Commit info
    lines.push(make_header_line("Commit".to_string(), LineType::Header));
    lines.push(make_header_line("─".repeat(40), LineType::Info));
    lines.push(make_header_line(format!("Hash:   {}", commit.hash), LineType::Context));
    lines.push(make_header_line(format!("Author: {}", commit.author), LineType::Context));
    lines.push(make_header_line(format!("Date:   {}", commit.date), LineType::Context));
    lines.push(make_header_line(String::new(), LineType::Context));
    lines.push(make_header_line(commit.subject.clone(), LineType::Info));
    lines.push(make_header_line(String::new(), LineType::Context));

    // PR info
    if let Some(pr) = pr {
        lines.push(make_header_line(String::new(), LineType::Context));
        lines.push(make_header_line("Pull Request".to_string(), LineType::Header));
        lines.push(make_header_line("─".repeat(40), LineType::Info));
        lines.push(make_header_line(format!("#{} {}", pr.number, pr.title), LineType::Info));
        lines.push(make_header_line(format!("State: {}", pr.state), LineType::Context));
        lines.push(make_header_line(format!("Author: {}", pr.author), LineType::Context));
        lines.push(make_header_line(format!("URL: {}", pr.url), LineType::Context));

        if !pr.body.is_empty() {
            lines.push(make_header_line(String::new(), LineType::Context));
            lines.push(make_header_line("Description".to_string(), LineType::Header));
            for line in pr.body.lines() {
                lines.push(make_header_line(format!("  {}", line), LineType::Context));
            }
        }

        // Reviews
        if !pr.reviews.is_empty() {
            lines.push(make_header_line(String::new(), LineType::Context));
            lines.push(make_header_line("Reviews".to_string(), LineType::Header));
            for review in &pr.reviews {
                let line_type = match review.state.as_str() {
                    "APPROVED" => LineType::Added,
                    "CHANGES_REQUESTED" => LineType::Removed,
                    _ => LineType::Context,
                };
                lines.push(make_header_line(format!("{} - {}", review.author, review.state), line_type));
                if !review.body.is_empty() {
                    for line in review.body.lines() {
                        lines.push(make_header_line(format!("  {}", line), LineType::Context));
                    }
                }
            }
        }
    } else {
        lines.push(make_header_line(String::new(), LineType::Context));
        lines.push(make_header_line("No PR found for this branch".to_string(), LineType::Info));
    }

    lines
}

/// Diff view widget
pub struct DiffView<'a> {
    colors: &'a Colors,
    focused: bool,
}

impl<'a> DiffView<'a> {
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

impl<'a> StatefulWidget for DiffView<'a> {
    type State = DiffViewState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = if self.focused {
            self.colors.style_border_focused()
        } else {
            self.colors.style_border()
        };

        let scroll_info = state.scroll_percent(area.height as usize);
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

        if state.lines.is_empty() {
            let msg = match &state.content {
                PreviewContent::Empty => "Select a file to view",
                _ => "No content",
            };
            let line = Line::from(Span::styled(msg, self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        state.ensure_visible(inner.height as usize);

        let visible_lines: Vec<_> = state
            .lines
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(inner.height as usize)
            .collect();

        let pane_width = ((inner.width as usize).saturating_sub(3)) / 2; // -3 for separator

        for (i, (idx, diff_line)) in visible_lines.into_iter().enumerate() {
            let y = inner.y + i as u16;
            let is_cursor = self.focused && idx == state.cursor;
            let line = render_diff_line(diff_line, is_cursor, self.colors, pane_width);
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

fn render_diff_line(diff_line: &DiffLine, cursor: bool, colors: &Colors, pane_width: usize) -> Line<'static> {
    let mut spans = vec![];

    // For headers, render full width
    if diff_line.is_header {
        let text = diff_line.left_text.as_deref().unwrap_or("");
        let style = match diff_line.line_type {
            LineType::Header => colors.style_header(),
            LineType::Info => colors.style_muted(),
            _ => ratatui::style::Style::default().fg(colors.text),
        };
        let content_style = if cursor {
            style.add_modifier(ratatui::style::Modifier::REVERSED)
        } else {
            style
        };
        spans.push(Span::styled(text.to_string(), content_style));
        return Line::from(spans);
    }

    let num_width = 4;

    // Left pane
    let left_num_str = diff_line.left_num
        .map(|n| format!("{:>width$}", n, width = num_width))
        .unwrap_or_else(|| " ".repeat(num_width));

    let left_text = diff_line.left_text.as_deref().unwrap_or("");
    let left_text = left_text.replace('\t', "    ");

    let left_style = match diff_line.line_type {
        LineType::Removed => colors.style_removed(),
        LineType::Context => ratatui::style::Style::default().fg(colors.text),
        _ => ratatui::style::Style::default().fg(colors.text),
    };

    let content_width = pane_width.saturating_sub(num_width + 3); // -3 for " │ "
    let left_content = truncate_or_pad(&left_text, content_width);

    // Right pane
    let right_num_str = diff_line.right_num
        .map(|n| format!("{:>width$}", n, width = num_width))
        .unwrap_or_else(|| " ".repeat(num_width));

    let right_text = diff_line.right_text.as_deref().unwrap_or("");
    let right_text = right_text.replace('\t', "    ");

    let right_style = match diff_line.line_type {
        LineType::Added => colors.style_added(),
        LineType::Context => ratatui::style::Style::default().fg(colors.text),
        _ => ratatui::style::Style::default().fg(colors.text),
    };

    let right_content = truncate_or_pad(&right_text, content_width);

    // Apply cursor highlight
    let left_style = if cursor {
        left_style.add_modifier(ratatui::style::Modifier::REVERSED)
    } else {
        left_style
    };
    let right_style = if cursor {
        right_style.add_modifier(ratatui::style::Modifier::REVERSED)
    } else {
        right_style
    };

    // Build line: left_num │ left_content │ right_num │ right_content
    spans.push(Span::styled(left_num_str, colors.style_muted()));
    spans.push(Span::styled(" │ ", colors.style_muted()));
    spans.push(Span::styled(left_content, left_style));
    spans.push(Span::styled(" │ ", colors.style_muted()));
    spans.push(Span::styled(right_num_str, colors.style_muted()));
    spans.push(Span::styled(" │ ", colors.style_muted()));
    spans.push(Span::styled(right_content, right_style));

    Line::from(spans)
}

fn truncate_or_pad(s: &str, width: usize) -> String {
    let char_count = s.chars().count();
    if char_count > width {
        s.chars().take(width.saturating_sub(1)).collect::<String>() + "…"
    } else {
        format!("{:width$}", s, width = width)
    }
}
