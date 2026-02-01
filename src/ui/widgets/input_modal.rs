use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::config::Colors;

/// Type of review action being performed
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewAction {
    Approve { pr_number: u64 },
    RequestChanges { pr_number: u64 },
    Comment { pr_number: u64 },
    LineComment { pr_number: u64, path: String, line: u32 },
}

impl ReviewAction {
    pub fn title(&self) -> String {
        match self {
            Self::Approve { pr_number } => format!("Approve PR #{}", pr_number),
            Self::RequestChanges { pr_number } => format!("Request Changes - PR #{}", pr_number),
            Self::Comment { pr_number } => format!("Comment on PR #{}", pr_number),
            Self::LineComment { pr_number, path, line } => {
                format!("Comment on {}:{} - PR #{}", path, line, pr_number)
            }
        }
    }

    pub fn needs_body(&self) -> bool {
        !matches!(self, Self::Approve { .. })
    }
}

/// Input modal state
#[derive(Debug, Default)]
pub struct InputModalState {
    pub visible: bool,
    pub action: Option<ReviewAction>,
    pub input: String,
    pub cursor_pos: usize,
    pub error: Option<String>,
}

impl InputModalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, action: ReviewAction) {
        self.visible = true;
        self.action = Some(action);
        self.input.clear();
        self.cursor_pos = 0;
        self.error = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.action = None;
        self.input.clear();
        self.cursor_pos = 0;
        self.error = None;
    }

    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// Handle key input, returns true if action should be submitted
    pub fn handle_key(&mut self, key: KeyEvent) -> InputResult {
        // Clear error on any input
        self.error = None;

        match key.code {
            KeyCode::Esc => {
                self.hide();
                InputResult::Cancelled
            }
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    || key.modifiers.contains(KeyModifiers::ALT)
                {
                    // Ctrl+Enter or Alt+Enter inserts newline
                    let byte_pos = self.input
                        .char_indices()
                        .nth(self.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(self.input.len());
                    self.input.insert(byte_pos, '\n');
                    self.cursor_pos += 1;
                    InputResult::Continue
                } else {
                    // Enter submits
                    if let Some(action) = &self.action {
                        if action.needs_body() && self.input.trim().is_empty() {
                            self.error = Some("Message cannot be empty".to_string());
                            InputResult::Continue
                        } else {
                            InputResult::Submit
                        }
                    } else {
                        InputResult::Submit
                    }
                }
            }
            KeyCode::Char(c) => {
                // For confirmation dialogs (no body needed), handle y/n specially
                let is_confirmation = self.action.as_ref().map(|a| !a.needs_body()).unwrap_or(false);
                if is_confirmation {
                    match c {
                        'y' | 'Y' => return InputResult::Submit,
                        'n' | 'N' => {
                            self.hide();
                            return InputResult::Cancelled;
                        }
                        _ => return InputResult::Continue, // Ignore other keys for confirmation
                    }
                }

                // For text input, insert character at cursor
                // Convert char index to byte index
                let byte_pos = self.input
                    .char_indices()
                    .nth(self.cursor_pos)
                    .map(|(i, _)| i)
                    .unwrap_or(self.input.len());
                self.input.insert(byte_pos, c);
                self.cursor_pos += 1;
                InputResult::Continue
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    // Convert char index to byte index
                    if let Some((byte_pos, _)) = self.input.char_indices().nth(self.cursor_pos) {
                        self.input.remove(byte_pos);
                    }
                }
                InputResult::Continue
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.input.chars().count() {
                    // Convert char index to byte index
                    if let Some((byte_pos, _)) = self.input.char_indices().nth(self.cursor_pos) {
                        self.input.remove(byte_pos);
                    }
                }
                InputResult::Continue
            }
            KeyCode::Left => {
                self.cursor_pos = self.cursor_pos.saturating_sub(1);
                InputResult::Continue
            }
            KeyCode::Right => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
                InputResult::Continue
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                InputResult::Continue
            }
            KeyCode::End => {
                self.cursor_pos = self.input.len();
                InputResult::Continue
            }
            _ => InputResult::Continue,
        }
    }

    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input)
    }
}

#[derive(Debug, PartialEq)]
pub enum InputResult {
    Continue,
    Submit,
    Cancelled,
}

/// Input modal widget
pub struct InputModal<'a> {
    colors: &'a Colors,
    state: &'a InputModalState,
}

impl<'a> InputModal<'a> {
    pub fn new(colors: &'a Colors, state: &'a InputModalState) -> Self {
        Self { colors, state }
    }
}

impl<'a> Widget for InputModal<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.state.visible {
            return;
        }

        let Some(action) = &self.state.action else {
            return;
        };

        // Clear background
        Clear.render(area, buf);

        let title = action.title();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.colors.style_border_focused())
            .title(Span::styled(title, self.colors.style_header()))
            .title_alignment(Alignment::Center);

        let inner = block.inner(area);
        block.render(area, buf);

        if action.needs_body() {
            // Show text input area
            let mut lines = vec![
                Line::from(Span::styled(
                    "Enter your message (Enter to submit, Esc to cancel):",
                    self.colors.style_muted(),
                )),
                Line::from(""),
            ];

            // Show input with cursor
            let input_lines: Vec<&str> = self.state.input.split('\n').collect();
            for line in &input_lines {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    ratatui::style::Style::default().fg(self.colors.text),
                )));
            }

            // Show cursor position indicator
            if input_lines.is_empty() || self.state.input.is_empty() {
                lines.push(Line::from(Span::styled(
                    "█",
                    ratatui::style::Style::default().fg(self.colors.text),
                )));
            }

            // Show error if any
            if let Some(error) = &self.state.error {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    error.clone(),
                    self.colors.style_removed(),
                )));
            }

            let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
            paragraph.render(inner, buf);
        } else {
            // Show confirmation prompt (for approve)
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "Are you sure you want to approve this PR?",
                    ratatui::style::Style::default().fg(self.colors.text),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Press ", self.colors.style_muted()),
                    Span::styled("y", self.colors.style_added()),
                    Span::styled(" to confirm, ", self.colors.style_muted()),
                    Span::styled("n", self.colors.style_removed()),
                    Span::styled(" or ", self.colors.style_muted()),
                    Span::styled("Esc", self.colors.style_muted()),
                    Span::styled(" to cancel", self.colors.style_muted()),
                ]),
            ];

            // Show error if any
            if let Some(error) = &self.state.error {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    error.clone(),
                    self.colors.style_removed(),
                )));
            }

            let paragraph = Paragraph::new(lines)
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false });
            paragraph.render(inner, buf);
        }
    }
}
