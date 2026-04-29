use crate::ui::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

pub fn render(frame: &mut Frame, area: Rect) {
    let modal = centered_rect(70, 80, area);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(theme::modal_border())
        .style(theme::modal_bg());
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let mut lines: Vec<Line<'static>> = Vec::new();
    lines.push(section("Movement"));
    lines.push(kv("j / k", "Line down / up"));
    lines.push(kv("Ctrl-d / Ctrl-u", "Half page"));
    lines.push(kv("Ctrl-f / Ctrl-b", "Full page"));
    lines.push(kv("g g / G", "Top / bottom"));
    lines.push(kv("] f / [ f", "Next / prev file"));
    lines.push(kv("] c / [ c", "Next / prev hunk"));
    lines.push(kv("] r / [ r", "Next / prev thread"));
    lines.push(Line::from(""));
    lines.push(section("Folding"));
    lines.push(kv("Space", "Toggle file under cursor"));
    lines.push(kv("z", "Toggle all (any open → close all, else open all)"));
    lines.push(Line::from(""));
    lines.push(section("Review"));
    lines.push(kv("c", "New comment on line under cursor"));
    lines.push(kv("r", "Reply to thread under cursor"));
    lines.push(kv("R", "Toggle resolved on thread"));
    lines.push(kv("e", "Edit draft under cursor"));
    lines.push(kv("d", "Delete draft under cursor"));
    lines.push(kv("o", "Open file in $EDITOR at relevant line"));
    lines.push(kv("y", "Yank review to clipboard (branch mode)"));
    lines.push(kv("Enter", "Apply review (PR mode)"));
    lines.push(Line::from(""));
    lines.push(section("Global"));
    lines.push(kv("?", "Toggle help"));
    lines.push(kv("Esc", "Close modal / cancel"));
    lines.push(kv("q", "Quit"));
    lines.push(kv("Ctrl-c", "Force quit"));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn section(name: &str) -> Line<'static> {
    Line::from(Span::styled(
        name.to_string(),
        theme::description_header().add_modifier(Modifier::UNDERLINED),
    ))
}

fn kv(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {key:<18}"), theme::comment_author()),
        Span::raw(desc.to_string()),
    ])
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
