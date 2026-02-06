mod parser;

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
use crate::ui::Highlighter;

use parser::{
    extract_diff_sides, is_binary, parse_diff,
    parse_hunk_header, truncate_or_pad, wrap_text, DiffLine, LineType,
};
use super::{Action, ReviewActionType};

/// What to show in the diff view
#[derive(Debug, Clone, Default)]
pub enum PreviewContent {
    #[default]
    Empty,
    FileDiff {
        path: String,
        content: String,
    },
    FolderDiff {
        path: String,
        content: String,
    },
}

/// Diff view mode: split (side-by-side) or unified (single pane)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffViewMode {
    #[default]
    Split,
    Unified,
}

/// Minimum width for split view (below this, auto-switch to unified)
const SPLIT_VIEW_MIN_WIDTH: u16 = 100;
/// Minimum width change to reset manual mode override
const RESIZE_THRESHOLD: u16 = 4;

/// Diff view widget state
pub struct DiffViewState {
    pub content: PreviewContent,
    pub lines: Vec<DiffLine>,
    pub cursor: usize,
    pub offset: usize,
    pub pr: Option<PrInfo>,
    pub view_mode: DiffViewMode,
    /// User manually set the view mode (don't auto-switch)
    manual_mode: bool,
    /// Last width seen (for detecting significant resize)
    last_width: u16,
    current_file: String,
    /// Syntax-highlighted lines for diff mode (left side, indexed by line number)
    highlighted_left: std::collections::HashMap<usize, Vec<(String, Style)>>,
    /// Syntax-highlighted lines for diff mode (right side, indexed by line number)
    highlighted_right: std::collections::HashMap<usize, Vec<(String, Style)>>,
}

impl Default for DiffViewState {
    fn default() -> Self {
        Self {
            content: PreviewContent::default(),
            lines: Vec::new(),
            cursor: 0,
            offset: 0,
            pr: None,
            view_mode: DiffViewMode::default(),
            manual_mode: false,
            last_width: 0,
            current_file: String::new(),
            highlighted_left: std::collections::HashMap::new(),
            highlighted_right: std::collections::HashMap::new(),
        }
    }
}

impl DiffViewState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_content(&mut self, content: PreviewContent) {
        self.set_content_with_highlighter(content, None);
    }

    pub fn set_content_highlighted(&mut self, content: PreviewContent, highlighter: &Highlighter) {
        self.set_content_with_highlighter(content, Some(highlighter));
    }

    fn set_content_with_highlighter(&mut self, content: PreviewContent, highlighter: Option<&Highlighter>) {
        // Store current file path for comment lookup
        self.current_file = match &content {
            PreviewContent::FileDiff { path, .. } => path.clone(),
            _ => String::new(),
        };

        // Clear previous highlighting
        self.highlighted_left.clear();
        self.highlighted_right.clear();

        // Apply syntax highlighting for diffs
        if let Some(h) = highlighter {
            if let PreviewContent::FileDiff { path, content: diff_text } = &content {
                let (left_lines, right_lines) = extract_diff_sides(diff_text);
                let left_highlighted = h.highlight_file(&left_lines.join("\n"), path);
                let right_highlighted = h.highlight_file(&right_lines.join("\n"), path);

                let mut left_line_num = 1usize;
                let mut right_line_num = 1usize;
                let mut left_idx = 0usize;
                let mut right_idx = 0usize;

                for line in diff_text.lines() {
                    if line.starts_with("@@") {
                        if let Some((l, r)) = parse_hunk_header(line) {
                            left_line_num = l;
                            right_line_num = r;
                        }
                    } else if line.starts_with('-') && !line.starts_with("---") {
                        if let Some(hl) = left_highlighted.get(left_idx) {
                            self.highlighted_left.insert(left_line_num, hl.clone());
                        }
                        left_line_num += 1;
                        left_idx += 1;
                    } else if line.starts_with('+') && !line.starts_with("+++") {
                        if let Some(hl) = right_highlighted.get(right_idx) {
                            self.highlighted_right.insert(right_line_num, hl.clone());
                        }
                        right_line_num += 1;
                        right_idx += 1;
                    } else if line.starts_with(' ') {
                        if let Some(hl) = left_highlighted.get(left_idx) {
                            self.highlighted_left.insert(left_line_num, hl.clone());
                        }
                        if let Some(hl) = right_highlighted.get(right_idx) {
                            self.highlighted_right.insert(right_line_num, hl.clone());
                        }
                        left_line_num += 1;
                        right_line_num += 1;
                        left_idx += 1;
                        right_idx += 1;
                    }
                }
            }
        }

        self.content = content;
        self.cursor = 0;
        self.offset = 0;
        self.parse_content();
    }

    pub fn set_pr(&mut self, pr: Option<PrInfo>) {
        self.pr = pr;
        // Re-parse to inject comments
        self.parse_content();
    }

    fn parse_content(&mut self) {
        let base_lines = match &self.content {
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
        };

        // Inject inline comments if we have PR info
        self.lines = self.inject_comments(base_lines);
    }

    fn inject_comments(&self, lines: Vec<DiffLine>) -> Vec<DiffLine> {
        let pr = match &self.pr {
            Some(pr) => pr,
            None => return lines,
        };

        let comments = match pr.file_comments.get(&self.current_file) {
            Some(c) => c,
            None => return lines,
        };

        if comments.is_empty() {
            return lines;
        }

        // Build comment threads: group replies with their parent comments
        let mut threads: std::collections::HashMap<u64, Vec<&crate::github::Comment>> = std::collections::HashMap::new();
        let mut root_comments: Vec<&crate::github::Comment> = Vec::new();

        for comment in comments {
            if let Some(parent_id) = comment.in_reply_to_id {
                threads.entry(parent_id).or_default().push(comment);
            } else {
                root_comments.push(comment);
            }
        }

        let mut result = Vec::with_capacity(lines.len() + comments.len() * 2);
        let wrap_width = 120; // Wrap comments at this width
        let mut rendered_comments: std::collections::HashSet<u64> = std::collections::HashSet::new();

        for line in lines {
            result.push(line.clone());

            // Check if there are root comments for this line
            for comment in &root_comments {
                // Skip already rendered comments
                if rendered_comments.contains(&comment.id) {
                    continue;
                }

                // Match comment to line based on side
                let matches = match comment.side.as_deref() {
                    Some("LEFT") => {
                        // Comment on old file - match against left_num or original_line
                        let target = comment.original_line.or(comment.line);
                        line.left_num.map(|n| n as u32) == target
                    }
                    Some("RIGHT") | None => {
                        // Comment on new file - match against right_num
                        // Default to RIGHT if side not specified (most common case)
                        line.right_num.map(|n| n as u32) == comment.line
                    }
                    _ => false,
                };

                if matches {
                    rendered_comments.insert(comment.id);

                    // Render the root comment
                    self.render_comment_to_lines(&mut result, comment, wrap_width, 0);

                    // Render any replies (threaded)
                    if let Some(replies) = threads.get(&comment.id) {
                        for reply in replies {
                            rendered_comments.insert(reply.id);
                            self.render_comment_to_lines(&mut result, reply, wrap_width, 1);
                        }
                    }
                }
            }
        }

        result
    }

    fn render_comment_to_lines(
        &self,
        result: &mut Vec<DiffLine>,
        comment: &crate::github::Comment,
        wrap_width: usize,
        indent_level: usize,
    ) {
        let indent = "  ".repeat(indent_level);
        let reply_indicator = if indent_level > 0 { "↳ " } else { "" };

        // Add comment header
        result.push(DiffLine {
            left_text: Some(format!("{}{}💬 {}", indent, reply_indicator, comment.author)),
            right_text: None,
            left_num: None,
            right_num: None,
            line_type: LineType::Comment,
            is_header: true,
        });

        // Add comment body lines with wrapping
        let body_indent = format!("{}   ", indent);
        for body_line in comment.body.lines() {
            for wrapped in wrap_text(body_line, wrap_width.saturating_sub(indent_level * 2)) {
                result.push(DiffLine {
                    left_text: Some(format!("{}{}", body_indent, wrapped)),
                    right_text: None,
                    left_num: None,
                    right_num: None,
                    line_type: LineType::Comment,
                    is_header: true,
                });
            }
        }
    }

    pub fn title(&self) -> String {
        match &self.content {
            PreviewContent::Empty => "Preview".to_string(),
            PreviewContent::FileDiff { path, .. } => path.clone(),
            PreviewContent::FolderDiff { path, .. } => format!("{}/", path),
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

    /// Click at a visible row (relative to inner area)
    pub fn click_at(&mut self, visible_row: usize) {
        let target = self.offset + visible_row;
        if target < self.lines.len() {
            self.cursor = target;
        }
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

    /// Get line number for current cursor position
    /// For diffs, always returns the new file line number (right side)
    /// For file content view, returns left_num
    pub fn get_current_line_number(&self) -> Option<usize> {
        self.lines.get(self.cursor).and_then(|line| {
            // Prefer right_num (new file) for diffs
            // Only fall back to left_num for file content view (where right is None)
            line.right_num.or_else(|| {
                // Only use left_num if this is a file content view (not a removed line in diff)
                if line.line_type == LineType::Context && line.right_text.is_none() {
                    line.left_num
                } else {
                    None
                }
            })
        })
    }

    /// Get the current file path being displayed
    pub fn get_current_file(&self) -> Option<&str> {
        match &self.content {
            PreviewContent::FileDiff { path, .. } => Some(path),
            _ => None,
        }
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

    /// Toggle between split and unified view modes (manual override)
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            DiffViewMode::Split => DiffViewMode::Unified,
            DiffViewMode::Unified => DiffViewMode::Split,
        };
        self.manual_mode = true;
    }

    /// Auto-adjust view mode based on available width (unless user manually set it)
    pub fn auto_adjust_view_mode(&mut self, width: u16) {
        // Reset manual mode on significant resize
        if self.manual_mode && self.last_width > 0 {
            let diff = (width as i32 - self.last_width as i32).unsigned_abs() as u16;
            if diff >= RESIZE_THRESHOLD {
                self.manual_mode = false;
            }
        }
        self.last_width = width;

        if self.manual_mode {
            return;
        }
        self.view_mode = if width < SPLIT_VIEW_MIN_WIDTH {
            DiffViewMode::Unified
        } else {
            DiffViewMode::Split
        };
    }

    /// Handle key input, return action for App to dispatch
    /// pr_number is needed for line comments
    pub fn handle_key(&mut self, key: &KeyEvent, pr_number: Option<u64>) -> Action {
        // Line comment
        if KeyInput::is_comment(key) {
            if let (Some(pr_num), Some(path), Some(line)) = (
                pr_number,
                self.get_current_file().map(|s| s.to_string()),
                self.get_current_line_number(),
            ) {
                return Action::OpenReviewModal(ReviewActionType::LineComment {
                    pr_number: pr_num,
                    path,
                    line: line as u32,
                });
            }
            return Action::None;
        }

        if KeyInput::is_down(key) {
            self.move_down();
            Action::None
        } else if KeyInput::is_up(key) {
            self.move_up();
            Action::None
        } else if KeyInput::is_fast_down(key) {
            self.move_down_n(5);
            Action::None
        } else if KeyInput::is_fast_up(key) {
            self.move_up_n(5);
            Action::None
        } else if KeyInput::is_page_down(key) {
            self.page_down(20);
            Action::None
        } else if KeyInput::is_page_up(key) {
            self.page_up(20);
            Action::None
        } else if KeyInput::is_top(key) {
            self.go_top();
            Action::None
        } else if KeyInput::is_bottom(key) {
            self.go_bottom();
            Action::None
        } else {
            Action::Ignored
        }
    }
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
        let border_style = self.colors.border_style(self.focused);

        // Build title with mode indicator and scroll info
        let mode_indicator = match state.view_mode {
            DiffViewMode::Split => "[split]",
            DiffViewMode::Unified => "[unified]",
        };
        let scroll_info = state.scroll_percent(area.height as usize);
        let title = if scroll_info.is_empty() {
            format!("{} {}", state.title(), mode_indicator)
        } else {
            format!("{} {} ─── {}", state.title(), mode_indicator, scroll_info)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        if state.lines.is_empty() {
            let (msg, hint) = match &state.content {
                PreviewContent::Empty => ("Select a file to view diff", "Press ? for help"),
                _ => ("No changes", ""),
            };
            let line = Line::from(Span::styled(msg, self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);

            if !hint.is_empty() && inner.height > 2 {
                let hint_line = Line::from(Span::styled(hint, self.colors.style_muted()));
                buf.set_line(inner.x, inner.y + 2, &hint_line, inner.width);
            }
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
        let has_diff_highlighting = !state.highlighted_left.is_empty() || !state.highlighted_right.is_empty();

        for (i, (idx, diff_line)) in visible_lines.into_iter().enumerate() {
            let y = inner.y + i as u16;
            let is_cursor = self.focused && idx == state.cursor;

            let line = if diff_line.is_header {
                render_header_line(diff_line, is_cursor, self.colors)
            } else if state.view_mode == DiffViewMode::Unified {
                let hl = match diff_line.line_type {
                    LineType::Added => diff_line.right_num.and_then(|n| state.highlighted_right.get(&n)),
                    LineType::Removed => diff_line.left_num.and_then(|n| state.highlighted_left.get(&n)),
                    _ => diff_line.right_num.and_then(|n| state.highlighted_right.get(&n))
                        .or_else(|| diff_line.left_num.and_then(|n| state.highlighted_left.get(&n))),
                };
                render_unified_diff_line(diff_line, hl, is_cursor, self.colors)
            } else if has_diff_highlighting {
                let left_hl = diff_line.left_num.and_then(|n| state.highlighted_left.get(&n));
                let right_hl = diff_line.right_num.and_then(|n| state.highlighted_right.get(&n));
                render_highlighted_diff_line(
                    diff_line,
                    left_hl,
                    right_hl,
                    is_cursor,
                    self.colors,
                    pane_width,
                )
            } else {
                render_diff_line(diff_line, is_cursor, self.colors, pane_width)
            };

            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

fn render_header_line(diff_line: &DiffLine, cursor: bool, colors: &Colors) -> Line<'static> {
    let text = diff_line.left_text.as_deref().unwrap_or("");

    let (style, prefix) = match diff_line.line_type {
        LineType::Header => (colors.style_header(), ""),
        LineType::Info => (colors.style_muted(), ""),
        LineType::Comment => {
            // Comments get a distinctive style with background
            let style = Style::default()
                .fg(colors.comment)
                .bg(colors.comment_bg);
            (style, "\u{2502} ")
        }
        _ => (Style::reset().fg(colors.text), ""),
    };

    let content_style = if cursor {
        style.add_modifier(ratatui::style::Modifier::REVERSED)
    } else {
        style
    };

    let display_text = format!("{}{}", prefix, text);
    Line::from(Span::styled(display_text, content_style))
}

fn render_highlighted_diff_line(
    diff_line: &DiffLine,
    left_hl: Option<&Vec<(String, Style)>>,
    right_hl: Option<&Vec<(String, Style)>>,
    cursor: bool,
    colors: &Colors,
    pane_width: usize,
) -> Line<'static> {
    let mut spans = vec![];
    let num_width = 4;
    let content_width = pane_width.saturating_sub(num_width + 1);

    // Determine background colors based on line type
    let (left_bg, right_bg) = match diff_line.line_type {
        LineType::Added => (None, Some(colors.added_bg)),
        LineType::Removed => (Some(colors.removed_bg), None),
        _ => (None, None),
    };

    // Left pane
    let left_num_str = diff_line.left_num
        .map(|n| format!("{:>width$}", n, width = num_width))
        .unwrap_or_else(|| " ".repeat(num_width));

    spans.push(Span::styled(left_num_str, colors.style_muted()));
    spans.push(Span::styled(" ", colors.style_muted()));

    // Left content with syntax highlighting
    let left_content_spans = if let Some(hl) = left_hl {
        build_highlighted_content(hl, content_width, left_bg, cursor)
    } else {
        let text = diff_line.left_text.as_deref().unwrap_or("");
        let text = truncate_or_pad(&text.replace('\t', "    "), content_width);
        let mut style = Style::reset().fg(colors.text);
        if let Some(bg) = left_bg {
            style = style.bg(bg);
        }
        if cursor {
            style = style.add_modifier(ratatui::style::Modifier::REVERSED);
        }
        vec![Span::styled(text, style)]
    };
    spans.extend(left_content_spans);

    // Separator
    spans.push(Span::styled(" │ ", colors.style_muted()));

    // Right pane
    let right_num_str = diff_line.right_num
        .map(|n| format!("{:>width$}", n, width = num_width))
        .unwrap_or_else(|| " ".repeat(num_width));

    spans.push(Span::styled(right_num_str, colors.style_muted()));
    spans.push(Span::styled(" ", colors.style_muted()));

    // Right content with syntax highlighting
    let right_content_spans = if let Some(hl) = right_hl {
        build_highlighted_content(hl, content_width, right_bg, cursor)
    } else {
        let text = diff_line.right_text.as_deref().unwrap_or("");
        let text = truncate_or_pad(&text.replace('\t', "    "), content_width);
        let mut style = Style::reset().fg(colors.text);
        if let Some(bg) = right_bg {
            style = style.bg(bg);
        }
        if cursor {
            style = style.add_modifier(ratatui::style::Modifier::REVERSED);
        }
        vec![Span::styled(text, style)]
    };
    spans.extend(right_content_spans);

    Line::from(spans)
}

fn build_highlighted_content(
    hl: &[(String, Style)],
    max_width: usize,
    bg_color: Option<ratatui::style::Color>,
    cursor: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut total_len = 0;

    for (text, style) in hl {
        let text = text.replace('\t', "    ");
        let remaining = max_width.saturating_sub(total_len);
        if remaining == 0 {
            break;
        }

        let display_text = if text.chars().count() > remaining {
            // Safe UTF-8 truncation using char boundaries
            let truncated: String = text.chars().take(remaining.saturating_sub(1)).collect();
            format!("{}…", truncated)
        } else {
            text.to_string()
        };

        total_len += display_text.chars().count();

        let mut final_style = *style;
        if let Some(bg) = bg_color {
            final_style = final_style.bg(bg);
        }
        if cursor {
            final_style = final_style.add_modifier(ratatui::style::Modifier::REVERSED);
        }

        spans.push(Span::styled(display_text, final_style));
    }

    // Pad to fill width
    if total_len < max_width {
        let padding = " ".repeat(max_width - total_len);
        let mut pad_style = Style::default();
        if let Some(bg) = bg_color {
            pad_style = pad_style.bg(bg);
        }
        if cursor {
            pad_style = pad_style.add_modifier(ratatui::style::Modifier::REVERSED);
        }
        spans.push(Span::styled(padding, pad_style));
    }

    spans
}

fn render_diff_line(diff_line: &DiffLine, cursor: bool, colors: &Colors, pane_width: usize) -> Line<'static> {
    let mut spans = vec![];

    // For headers and comments, render full width
    if diff_line.is_header {
        let text = diff_line.left_text.as_deref().unwrap_or("");
        let style = match diff_line.line_type {
            LineType::Header => colors.style_header(),
            LineType::Info => colors.style_muted(),
            LineType::Comment => ratatui::style::Style::reset().fg(colors.comment),
            _ => ratatui::style::Style::reset().fg(colors.text),
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

    // Single column mode (file content view, not diff)
    let is_single_column = diff_line.right_text.is_none() && diff_line.right_num.is_none();

    if is_single_column {
        let left_num_str = diff_line.left_num
            .map(|n| format!("{:>width$}", n, width = num_width))
            .unwrap_or_else(|| " ".repeat(num_width));

        let left_text = diff_line.left_text.as_deref().unwrap_or("");
        let left_text = left_text.replace('\t', "    ");

        let style = ratatui::style::Style::reset().fg(colors.text);
        let content_style = if cursor {
            style.add_modifier(ratatui::style::Modifier::REVERSED)
        } else {
            style
        };

        spans.push(Span::styled(left_num_str, colors.style_muted()));
        spans.push(Span::styled(" ", colors.style_muted()));
        spans.push(Span::styled(left_text, content_style));
        return Line::from(spans);
    }

    // Side-by-side diff mode
    // Left pane
    let left_num_str = diff_line.left_num
        .map(|n| format!("{:>width$}", n, width = num_width))
        .unwrap_or_else(|| " ".repeat(num_width));

    let left_text = diff_line.left_text.as_deref().unwrap_or("");
    let left_text = left_text.replace('\t', "    ");

    let left_style = match diff_line.line_type {
        LineType::Removed => colors.style_removed(),
        LineType::Context => ratatui::style::Style::reset().fg(colors.text),
        _ => ratatui::style::Style::reset().fg(colors.text),
    };

    let content_width = pane_width.saturating_sub(num_width + 1); // -1 for " "
    let left_content = truncate_or_pad(&left_text, content_width);

    // Right pane
    let right_num_str = diff_line.right_num
        .map(|n| format!("{:>width$}", n, width = num_width))
        .unwrap_or_else(|| " ".repeat(num_width));

    let right_text = diff_line.right_text.as_deref().unwrap_or("");
    let right_text = right_text.replace('\t', "    ");

    let right_style = match diff_line.line_type {
        LineType::Added => colors.style_added(),
        LineType::Context => ratatui::style::Style::reset().fg(colors.text),
        _ => ratatui::style::Style::reset().fg(colors.text),
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

    // Build line: left_num  left_content │ right_num  right_content
    spans.push(Span::styled(left_num_str, colors.style_muted()));
    spans.push(Span::styled(" ", colors.style_muted()));
    spans.push(Span::styled(left_content, left_style));
    spans.push(Span::styled(" │ ", colors.style_muted()));
    spans.push(Span::styled(right_num_str, colors.style_muted()));
    spans.push(Span::styled(" ", colors.style_muted()));
    spans.push(Span::styled(right_content, right_style));

    Line::from(spans)
}

/// Render a diff line in unified mode (single pane, traditional +/- prefix)
fn render_unified_diff_line(
    diff_line: &DiffLine,
    highlight: Option<&Vec<(String, Style)>>,
    cursor: bool,
    colors: &Colors,
) -> Line<'static> {
    let mut spans = vec![];
    let num_width = 4;

    // Show appropriate line number and prefix based on line type
    let (prefix, line_num, text, base_style, bg_color) = match diff_line.line_type {
        LineType::Added => {
            let num = diff_line.right_num
                .map(|n| format!("{:>width$}", n, width = num_width))
                .unwrap_or_else(|| " ".repeat(num_width));
            let text = diff_line.right_text.as_deref().unwrap_or("");
            ("+", num, text, colors.style_added(), Some(colors.added_bg))
        }
        LineType::Removed => {
            let num = diff_line.left_num
                .map(|n| format!("{:>width$}", n, width = num_width))
                .unwrap_or_else(|| " ".repeat(num_width));
            let text = diff_line.left_text.as_deref().unwrap_or("");
            ("-", num, text, colors.style_removed(), Some(colors.removed_bg))
        }
        LineType::Context => {
            let num = diff_line.right_num.or(diff_line.left_num)
                .map(|n| format!("{:>width$}", n, width = num_width))
                .unwrap_or_else(|| " ".repeat(num_width));
            let text = diff_line.right_text.as_deref()
                .or(diff_line.left_text.as_deref())
                .unwrap_or("");
            (" ", num, text, Style::reset().fg(colors.text), None)
        }
        _ => {
            let num = " ".repeat(num_width);
            let text = diff_line.left_text.as_deref().unwrap_or("");
            (" ", num, text, Style::reset().fg(colors.text), None)
        }
    };

    spans.push(Span::styled(line_num, colors.style_muted()));
    spans.push(Span::styled(" ", colors.style_muted()));

    // Prefix with base style
    let prefix_style = if cursor {
        base_style.add_modifier(ratatui::style::Modifier::REVERSED)
    } else {
        base_style
    };
    spans.push(Span::styled(prefix.to_string(), prefix_style));

    // Content with syntax highlighting if available
    if let Some(hl) = highlight {
        for (hl_text, hl_style) in hl {
            let text = hl_text.replace('\t', "    ");
            let mut style = *hl_style;
            if let Some(bg) = bg_color {
                style = style.bg(bg);
            }
            if cursor {
                style = style.add_modifier(ratatui::style::Modifier::REVERSED);
            }
            spans.push(Span::styled(text, style));
        }
    } else {
        let text = text.replace('\t', "    ");
        let content_style = if cursor {
            base_style.add_modifier(ratatui::style::Modifier::REVERSED)
        } else {
            base_style
        };
        spans.push(Span::styled(text, content_style));
    }

    Line::from(spans)
}
