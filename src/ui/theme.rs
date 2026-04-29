use ratatui::style::{Color, Style};

pub fn diff_add() -> Style {
    Style::default().fg(Color::Green)
}

pub fn diff_remove() -> Style {
    Style::default().fg(Color::Red)
}

pub fn hunk_header() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn file_header() -> Style {
    Style::default().fg(Color::White)
}

pub fn comment_author() -> Style {
    Style::default().fg(Color::Yellow)
}

pub fn draft_marker() -> Style {
    Style::default().fg(Color::Magenta)
}

pub fn outdated() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn resolved() -> Style {
    Style::default().fg(Color::DarkGray)
}
