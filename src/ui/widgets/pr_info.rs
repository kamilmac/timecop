use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::config::Colors;
use crate::github::PrSummary;

/// PR list panel widget state
#[derive(Debug, Default)]
pub struct PrListPanelState {
    pub prs: Vec<PrSummary>,
    pub cursor: usize,
    pub offset: usize,
    pub loading: bool,
    pub current_branch: String,
}

impl PrListPanelState {
    pub fn new() -> Self {
        Self {
            loading: true,
            ..Default::default()
        }
    }

    pub fn set_prs(&mut self, prs: Vec<PrSummary>) {
        self.prs = prs;
        self.loading = false;

        // Try to select the PR for current branch
        if let Some(idx) = self.prs.iter().position(|pr| pr.branch == self.current_branch) {
            self.cursor = idx;
        } else if self.cursor >= self.prs.len() && !self.prs.is_empty() {
            self.cursor = 0;
        }
    }

    pub fn set_current_branch(&mut self, branch: String) {
        self.current_branch = branch;

        // Auto-select current branch's PR if available
        if let Some(idx) = self.prs.iter().position(|pr| pr.branch == self.current_branch) {
            self.cursor = idx;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor < self.prs.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn selected(&self) -> Option<&PrSummary> {
        self.prs.get(self.cursor)
    }

    pub fn selected_number(&self) -> Option<u64> {
        self.selected().map(|pr| pr.number)
    }

    fn ensure_visible(&mut self, height: usize) {
        // 2 lines per PR
        let visible_prs = height / 2;
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + visible_prs {
            self.offset = self.cursor.saturating_sub(visible_prs) + 1;
        }
    }
}

/// PR list panel widget
pub struct PrListPanel<'a> {
    colors: &'a Colors,
    focused: bool,
}

impl<'a> PrListPanel<'a> {
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

impl<'a> StatefulWidget for PrListPanel<'a> {
    type State = PrListPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = if self.focused {
            self.colors.style_border_focused()
        } else {
            self.colors.style_border()
        };

        let title = if state.loading && state.prs.is_empty() {
            "PRs (loading...)".to_string()
        } else if state.loading {
            format!("PRs ({}) ↻", state.prs.len())
        } else {
            format!("PRs ({})", state.prs.len())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        // Only show "Loading..." if we have no PRs yet
        if state.loading && state.prs.is_empty() {
            let line = Line::from(Span::styled("Loading...", self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        if state.prs.is_empty() {
            let line = Line::from(Span::styled("No open PRs", self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        state.ensure_visible(inner.height as usize);

        // 2 lines per PR
        let visible_prs = (inner.height as usize) / 2;

        for (i, pr) in state
            .prs
            .iter()
            .skip(state.offset)
            .take(visible_prs)
            .enumerate()
        {
            let y = inner.y + (i * 2) as u16;
            let idx = state.offset + i;
            let is_selected = self.focused && idx == state.cursor;
            let is_current_branch = pr.branch == state.current_branch;

            let (line1, line2) = render_pr_lines(pr, is_selected, is_current_branch, self.colors, inner.width as usize);
            buf.set_line(inner.x, y, &line1, inner.width);
            if y + 1 < inner.y + inner.height {
                buf.set_line(inner.x, y + 1, &line2, inner.width);
            }
        }
    }
}

fn render_pr_lines(
    pr: &PrSummary,
    selected: bool,
    is_current_branch: bool,
    colors: &Colors,
    width: usize,
) -> (Line<'static>, Line<'static>) {
    // Line 1: indicators #number @author days_ago
    let mut spans1 = vec![];

    // Current branch indicator
    let branch_indicator = if is_current_branch { "●" } else { " " };
    let branch_style = if is_current_branch {
        colors.style_added()
    } else {
        colors.style_muted()
    };
    spans1.push(Span::styled(branch_indicator.to_string(), branch_style));

    // Review requested indicator
    let review_indicator = if pr.review_requested { "◆" } else { " " };
    let review_style = if pr.review_requested {
        ratatui::style::Style::default().fg(ratatui::style::Color::Yellow)
    } else {
        colors.style_muted()
    };
    spans1.push(Span::styled(review_indicator.to_string(), review_style));

    // PR number
    let pr_num = format!("#{:<5}", pr.number);
    let num_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else {
        colors.style_muted()
    };
    spans1.push(Span::styled(pr_num, num_style));

    // Author
    let author = format!("@{}", pr.author);
    let author_len = author.len();
    let author_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else if is_current_branch {
        colors.style_header()
    } else {
        ratatui::style::Style::default().fg(colors.text)
    };
    spans1.push(Span::styled(author, author_style));

    // Days ago (right-aligned)
    let days_ago = days_ago_from_date(&pr.updated_at);
    let days_str = format!(" {}", days_ago);
    let used = 2 + 7 + author_len; // indicators (2) + #number + author
    let padding = width.saturating_sub(used + days_str.len());
    if padding > 0 {
        let pad_style = if selected {
            colors.style_selected().add_modifier(Modifier::REVERSED)
        } else {
            ratatui::style::Style::default()
        };
        spans1.push(Span::styled(" ".repeat(padding), pad_style));
    }
    let days_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else {
        colors.style_muted()
    };
    spans1.push(Span::styled(days_str, days_style));

    // Line 2: indented title
    let mut spans2 = vec![];
    let indent = "   "; // 3 spaces
    let title_width = width.saturating_sub(indent.len());
    let title = truncate(&pr.title, title_width);

    let title_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else if is_current_branch {
        colors.style_header()
    } else {
        ratatui::style::Style::default().fg(colors.text)
    };

    spans2.push(Span::styled(indent.to_string(), title_style));
    spans2.push(Span::styled(title, title_style));

    (Line::from(spans1), Line::from(spans2))
}

fn days_ago_from_date(date_str: &str) -> String {
    // Parse YYYY-MM-DD format
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }

    let year: i32 = parts[0].parse().unwrap_or(0);
    let month: u32 = parts[1].parse().unwrap_or(0);
    let day: u32 = parts[2].parse().unwrap_or(0);

    if year == 0 || month == 0 || day == 0 {
        return date_str.to_string();
    }

    // Get current date (simple approach using system time)
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert to days since epoch (approximate)
    let now_days = now / 86400;

    // Calculate date's days since epoch (approximate, ignoring leap years for simplicity)
    let date_days = (year as u64 - 1970) * 365
        + (year as u64 - 1969) / 4  // leap years
        + days_before_month(month)
        + day as u64
        - 1;

    let diff = now_days.saturating_sub(date_days);

    if diff == 0 {
        "today".to_string()
    } else if diff == 1 {
        "1d ago".to_string()
    } else if diff < 7 {
        format!("{}d ago", diff)
    } else if diff < 30 {
        format!("{}w ago", diff / 7)
    } else {
        format!("{}mo ago", diff / 30)
    }
}

fn days_before_month(month: u32) -> u64 {
    match month {
        1 => 0,
        2 => 31,
        3 => 59,
        4 => 90,
        5 => 120,
        6 => 151,
        7 => 181,
        8 => 212,
        9 => 243,
        10 => 273,
        11 => 304,
        12 => 334,
        _ => 0,
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        s.chars().take(max.saturating_sub(1)).collect::<String>() + "…"
    } else {
        s.to_string()
    }
}
