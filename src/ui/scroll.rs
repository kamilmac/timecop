pub struct ScrollState {
    pub cursor: usize,
    pub offset: usize,
}

impl ScrollState {
    pub fn new() -> Self {
        Self { cursor: 0, offset: 0 }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}
