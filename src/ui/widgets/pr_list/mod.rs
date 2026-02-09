use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::config::Colors;
use crate::event::KeyInput;
use crate::github::PrSummary;

use super::{Action, ReviewAction, ScrollState};

/// PR list panel widget state
#[derive(Debug, Default)]
pub struct PrListPanelState {
    pub prs: Vec<PrSummary>,
    pub scroll: ScrollState,
    pub loading: bool,
    pub current_branch: String,
    pub gh_available: bool,
}

impl PrListPanelState {
    pub fn new() -> Self {
        Self {
            loading: true,
            gh_available: true, // Assume available until told otherwise
            ..Default::default()
        }
    }

    pub fn set_gh_available(&mut self, available: bool) {
        self.gh_available = available;
        if !available {
            self.loading = false;
        }
    }

    pub fn set_prs(&mut self, prs: Vec<PrSummary>) {
        self.prs = prs;
        self.loading = false;
        self.scroll.set_len(self.prs.len());

        // Try to select the PR for current branch
        if let Some(idx) = self.prs.iter().position(|pr| pr.branch == self.current_branch) {
            self.scroll.cursor = idx;
        }
    }

    pub fn set_current_branch(&mut self, branch: String) {
        self.current_branch = branch;

        // Auto-select current branch's PR if available
        if let Some(idx) = self.prs.iter().position(|pr| pr.branch == self.current_branch) {
            self.scroll.cursor = idx;
        }
    }

    /// Click at a visible row (relative to inner area). Returns true if selection changed.
    pub fn click_at(&mut self, visible_row: usize) -> bool {
        let target = self.scroll.offset + visible_row;
        if target < self.prs.len() && target != self.scroll.cursor {
            self.scroll.cursor = target;
            return true;
        }
        false
    }

    pub fn selected(&self) -> Option<&PrSummary> {
        self.prs.get(self.scroll.cursor)
    }

    pub fn selected_number(&self) -> Option<u64> {
        self.selected().map(|pr| pr.number)
    }

    /// Handle key input, return action for App to dispatch
    pub fn handle_key(&mut self, key: &KeyEvent) -> Action {
        // All PR actions require gh CLI
        if !self.gh_available {
            return Action::Ignored;
        }

        // Review actions
        if KeyInput::is_approve(key) {
            if let Some(pr) = self.selected() {
                return Action::OpenReviewModal(ReviewAction::Approve {
                    pr_number: pr.number,
                });
            }
            return Action::None;
        }

        if KeyInput::is_request_changes(key) {
            if let Some(pr) = self.selected() {
                return Action::OpenReviewModal(ReviewAction::RequestChanges {
                    pr_number: pr.number,
                });
            }
            return Action::None;
        }

        if KeyInput::is_comment(key) {
            if let Some(pr) = self.selected() {
                return Action::OpenReviewModal(ReviewAction::Comment {
                    pr_number: pr.number,
                });
            }
            return Action::None;
        }

        if KeyInput::is_down(key) {
            self.scroll.move_down();
            if let Some(pr) = self.selected() {
                return Action::PrSelected(pr.number);
            }
            return Action::None;
        }

        if KeyInput::is_up(key) {
            self.scroll.move_up();
            if let Some(pr) = self.selected() {
                return Action::PrSelected(pr.number);
            }
            return Action::None;
        }

        if KeyInput::is_enter(key) {
            if let Some(pr) = self.selected() {
                return Action::CheckoutPr(pr.number);
            }
            return Action::None;
        }

        // Note: 'o' (open in browser) is handled by App directly

        Action::Ignored
    }
}

/// PR list panel widget
pub struct PrListPanel<'a> {
    colors: &'a Colors,
    focused: bool,
    spinner_frame: usize,
}

impl<'a> PrListPanel<'a> {
    pub fn new(colors: &'a Colors) -> Self {
        Self {
            colors,
            focused: false,
            spinner_frame: 0,
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn spinner_frame(mut self, frame: usize) -> Self {
        self.spinner_frame = frame;
        self
    }
}

impl<'a> StatefulWidget for PrListPanel<'a> {
    type State = PrListPanelState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = self.colors.border_style(self.focused);

        const SPINNER: [char; 4] = ['◐', '◓', '◑', '◒'];
        let spinner = SPINNER[self.spinner_frame % SPINNER.len()];

        let title = if state.loading && state.prs.is_empty() {
            format!("PRs {}", spinner)
        } else if state.loading {
            format!("PRs ({}) {}", state.prs.len(), spinner)
        } else {
            format!("PRs ({})", state.prs.len())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        if !state.gh_available {
            let line = Line::from(Span::styled("gh CLI not found", self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            let hint = Line::from(Span::styled("cli.github.com", self.colors.style_muted()));
            if inner.height > 1 {
                buf.set_line(inner.x, inner.y + 1, &hint, inner.width);
            }
            return;
        }

        if state.prs.is_empty() && !state.loading {
            let line = Line::from(Span::styled("No open PRs", self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        let visible_count = inner.height as usize;
        state.scroll.ensure_visible(visible_count);

        for (i, pr) in state
            .prs
            .iter()
            .skip(state.scroll.offset)
            .take(visible_count)
            .enumerate()
        {
            let y = inner.y + i as u16;
            let idx = state.scroll.offset + i;
            let is_selected = self.focused && idx == state.scroll.cursor;
            let is_current_branch = pr.branch == state.current_branch;

            let line = render_pr_line(pr, is_selected, is_current_branch, self.colors, inner.width as usize);
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

fn render_pr_line(
    pr: &PrSummary,
    selected: bool,
    is_current_branch: bool,
    colors: &Colors,
    width: usize,
) -> Line<'static> {
    let mut spans = vec![];

    // Current branch indicator
    let branch_indicator = if is_current_branch { "●" } else { " " };
    let branch_style = if is_current_branch {
        colors.style_added()
    } else {
        colors.style_muted()
    };
    spans.push(Span::styled(branch_indicator.to_string(), branch_style));

    // Review requested indicator
    let review_indicator = if pr.review_requested { "◆" } else { " " };
    let review_style = if pr.review_requested {
        ratatui::style::Style::reset().fg(ratatui::style::Color::Yellow)
    } else {
        colors.style_muted()
    };
    spans.push(Span::styled(format!("{} ", review_indicator), review_style));

    // PR number
    let pr_num = format!("#{:<5}", pr.number);
    let num_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else {
        colors.style_muted()
    };
    spans.push(Span::styled(pr_num, num_style));

    // Separator
    let sep_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else {
        colors.style_muted()
    };
    spans.push(Span::styled("│ ", sep_style));

    // Author (fixed width)
    let author = format!("{:<12}", truncate(&pr.author, 12));
    let author_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else if is_current_branch {
        colors.style_header()
    } else {
        ratatui::style::Style::reset().fg(colors.text)
    };
    spans.push(Span::styled(author, author_style));

    // Separator
    spans.push(Span::styled("│ ", sep_style));

    // Days ago (fixed width, right side)
    let days_ago = format!("{:>7}", days_ago_from_date(&pr.updated_at));

    // Title (fills remaining space)
    // Calculate: indicators(4) + #number(7) + sep(2) + author(12) + sep(2) + days(7) + sep(2) = 36
    let fixed_width = 4 + 7 + 2 + 12 + 2 + 7 + 2;
    let title_width = width.saturating_sub(fixed_width);
    let title = truncate(&pr.title, title_width);
    let title_padded = format!("{:<width$}", title, width = title_width);

    let title_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else if is_current_branch {
        colors.style_header()
    } else {
        ratatui::style::Style::reset().fg(colors.text)
    };
    spans.push(Span::styled(title_padded, title_style));

    // Separator before date
    spans.push(Span::styled("│ ", sep_style));

    // Days ago
    let days_style = if selected {
        colors.style_selected().add_modifier(Modifier::REVERSED)
    } else {
        colors.style_muted()
    };
    spans.push(Span::styled(days_ago, days_style));

    Line::from(spans)
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
        "1d".to_string()
    } else if diff < 7 {
        format!("{}d", diff)
    } else if diff < 30 {
        format!("{}w", diff / 7)
    } else {
        format!("{}mo", diff / 30)
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
