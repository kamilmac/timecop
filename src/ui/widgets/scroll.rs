/// Shared scroll state for widgets with cursor + viewport offset.
#[derive(Debug, Default, Clone)]
pub struct ScrollState {
    pub cursor: usize,
    pub offset: usize,
    len: usize,
}

impl ScrollState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the total number of items. Clamps cursor if needed.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        if self.cursor >= len && len > 0 {
            self.cursor = len - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor < self.len.saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down_n(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.len.saturating_sub(1));
    }

    pub fn move_up_n(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    pub fn go_top(&mut self) {
        self.cursor = 0;
        self.offset = 0;
    }

    pub fn go_bottom(&mut self) {
        self.cursor = self.len.saturating_sub(1);
    }

    /// Adjust offset so cursor is visible within `visible_height` rows.
    pub fn ensure_visible(&mut self, visible_height: usize) {
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + visible_height {
            self.offset = self.cursor.saturating_sub(visible_height) + 1;
        }
    }

    /// Click at a visible row (relative to inner area).
    pub fn click_at(&mut self, visible_row: usize) {
        let target = self.offset + visible_row;
        if target < self.len {
            self.cursor = target;
        }
    }

    pub fn scroll_percent(&self, visible_height: usize) -> String {
        if self.len == 0 || self.len <= visible_height {
            return String::new();
        }
        let percent = (self.offset * 100) / self.len.saturating_sub(visible_height).max(1);
        format!("{}%", percent.min(100))
    }
}
