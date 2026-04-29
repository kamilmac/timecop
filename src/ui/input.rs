use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub enum InputTarget {
    NewComment { file: String, line: u32 },
    Reply { thread_id: String },
    EditDraft { draft_idx: usize },
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub target: InputTarget,
    pub buffer: String,
}

impl InputState {
    pub fn new(target: InputTarget) -> Self {
        Self { target, buffer: String::new() }
    }

    pub fn with_body(target: InputTarget, body: String) -> Self {
        Self { target, buffer: body }
    }
}

pub enum InputResult {
    Continue,
    Submit(String),
    Cancel,
}

pub fn handle_key(state: &mut InputState, key: KeyEvent) -> InputResult {
    match key.code {
        KeyCode::Esc => InputResult::Cancel,
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            InputResult::Submit(state.buffer.clone())
        }
        KeyCode::Char(c) => {
            state.buffer.push(c);
            InputResult::Continue
        }
        KeyCode::Enter => {
            state.buffer.push('\n');
            InputResult::Continue
        }
        KeyCode::Backspace => {
            state.buffer.pop();
            InputResult::Continue
        }
        _ => InputResult::Continue,
    }
}
