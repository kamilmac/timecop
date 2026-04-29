pub struct InputState {
    pub buffer: String,
    pub active: bool,
}

impl InputState {
    pub fn new() -> Self {
        Self { buffer: String::new(), active: false }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}
