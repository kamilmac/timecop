use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::config::Colors;
use crate::git::Commit;

/// Commit list widget state
#[derive(Debug, Default)]
pub struct CommitListState {
    pub commits: Vec<Commit>,
    pub cursor: usize,
}

impl CommitListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_commits(&mut self, commits: Vec<Commit>) {
        self.commits = commits;
        if self.cursor >= self.commits.len() && !self.commits.is_empty() {
            self.cursor = self.commits.len() - 1;
        }
    }

    pub fn selected(&self) -> Option<&Commit> {
        self.commits.get(self.cursor)
    }

    pub fn move_down(&mut self) {
        if self.cursor < self.commits.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }
}

/// Commit list widget
pub struct CommitList<'a> {
    colors: &'a Colors,
    focused: bool,
}

impl<'a> CommitList<'a> {
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

impl<'a> StatefulWidget for CommitList<'a> {
    type State = CommitListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = if self.focused {
            self.colors.style_border_focused()
        } else {
            self.colors.style_border()
        };

        let title = format!("Commits ({})", state.commits.len());
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        if state.commits.is_empty() {
            let line = Line::from(Span::styled("No commits", self.colors.style_muted()));
            buf.set_line(inner.x, inner.y, &line, inner.width);
            return;
        }

        for (i, commit) in state.commits.iter().enumerate().take(inner.height as usize) {
            let y = inner.y + i as u16;
            let is_selected = self.focused && i == state.cursor;
            let line = render_commit(commit, is_selected, self.colors, inner.width as usize);
            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

fn render_commit(commit: &Commit, selected: bool, colors: &Colors, width: usize) -> Line<'static> {
    let mut spans = vec![];

    // Cursor
    let cursor = if selected { "> " } else { "  " };
    spans.push(Span::raw(cursor.to_string()));

    // Short hash
    spans.push(Span::styled(
        commit.short_hash.clone(),
        colors.style_muted(),
    ));
    spans.push(Span::raw(" ".to_string()));

    // Subject (truncate if needed)
    let available = width.saturating_sub(cursor.len() + 8); // 7 for hash + 1 space
    let subject = if commit.subject.len() > available {
        format!("{}...", &commit.subject[..available.saturating_sub(3)])
    } else {
        commit.subject.clone()
    };

    let subject_style = if selected {
        colors.style_selected()
    } else {
        ratatui::style::Style::default().fg(colors.text)
    };
    spans.push(Span::styled(subject, subject_style));

    Line::from(spans)
}
