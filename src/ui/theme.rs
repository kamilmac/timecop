use ratatui::style::{Color, Modifier, Style};

pub fn diff_add_bg() -> Style {
    Style::default().bg(Color::Rgb(20, 50, 25))
}

pub fn diff_remove_bg() -> Style {
    Style::default().bg(Color::Rgb(60, 25, 30))
}

pub fn diff_add_marker() -> Style {
    Style::default().fg(Color::Rgb(120, 200, 130))
}

pub fn diff_remove_marker() -> Style {
    Style::default().fg(Color::Rgb(220, 120, 130))
}

pub fn hunk_header() -> Style {
    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
}

pub fn file_header() -> Style {
    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
}

pub fn file_header_collapsed() -> Style {
    Style::default().fg(Color::Gray)
}

pub fn additions() -> Style {
    Style::default().fg(Color::Green)
}

pub fn deletions() -> Style {
    Style::default().fg(Color::Red)
}

pub fn comment_author() -> Style {
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
}

pub fn comment_body() -> Style {
    Style::default().fg(Color::White)
}

pub fn draft_marker() -> Style {
    Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
}

pub fn outdated() -> Style {
    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
}

pub fn resolved() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn description_header() -> Style {
    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
}

pub fn gutter() -> Style {
    Style::default().fg(Color::DarkGray)
}

pub fn cursor_line() -> Style {
    Style::default().bg(Color::Rgb(50, 50, 70))
}

pub fn status_bar() -> Style {
    Style::default().bg(Color::Rgb(30, 30, 40)).fg(Color::White)
}

pub fn warning() -> Style {
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
}

pub fn modal_border() -> Style {
    Style::default().fg(Color::Cyan)
}

pub fn modal_bg() -> Style {
    Style::default().bg(Color::Rgb(20, 20, 30))
}

pub fn thread_bar() -> Style {
    Style::default().fg(Color::Rgb(120, 140, 180))
}
