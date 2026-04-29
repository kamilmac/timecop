#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pub cursor: usize,
    pub offset: usize,
}

impl ScrollState {
    pub fn new() -> Self {
        Self { cursor: 0, offset: 0 }
    }

    pub fn ensure_visible(&mut self, viewport_height: usize, total: usize) {
        if total == 0 {
            self.cursor = 0;
            self.offset = 0;
            return;
        }
        if self.cursor >= total {
            self.cursor = total - 1;
        }
        let margin = 3usize.min(viewport_height / 4);
        if self.cursor < self.offset + margin {
            self.offset = self.cursor.saturating_sub(margin);
        } else if self.cursor + margin >= self.offset + viewport_height {
            self.offset = self.cursor + margin + 1 - viewport_height;
        }
        let max_offset = total.saturating_sub(viewport_height);
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }

    pub fn move_by(&mut self, delta: isize, total: usize) {
        if total == 0 {
            return;
        }
        let new = (self.cursor as isize + delta).clamp(0, (total - 1) as isize);
        self.cursor = new as usize;
    }

    pub fn jump_to(&mut self, idx: usize, total: usize) {
        if total == 0 {
            return;
        }
        self.cursor = idx.min(total - 1);
    }
}
