use ratatui::style::{Color, Modifier, Style};

const SUMI_INK_1: Color = Color::Rgb(0x1F, 0x1F, 0x28);
const SUMI_INK_2: Color = Color::Rgb(0x2A, 0x2A, 0x37);
const SUMI_INK_4: Color = Color::Rgb(0x54, 0x54, 0x6D);
const FUJI_WHITE: Color = Color::Rgb(0xDC, 0xD7, 0xBA);
const FUJI_GRAY: Color = Color::Rgb(0x72, 0x71, 0x69);
const KATANA_GRAY: Color = Color::Rgb(0x71, 0x7C, 0x7C);
const DIFF_ADD_BG: Color = Color::Rgb(0x2B, 0x33, 0x28);
const DIFF_DELETE_BG: Color = Color::Rgb(0x43, 0x24, 0x2B);
const SPRING_GREEN: Color = Color::Rgb(0x98, 0xBB, 0x6C);
const AUTUMN_RED: Color = Color::Rgb(0xC3, 0x40, 0x43);
const AUTUMN_YELLOW: Color = Color::Rgb(0xDC, 0xA5, 0x61);
const CRYSTAL_BLUE: Color = Color::Rgb(0x7E, 0x9C, 0xD8);
const WAVE_AQUA_1: Color = Color::Rgb(0x6A, 0x95, 0x89);
const WAVE_AQUA_2: Color = Color::Rgb(0x7A, 0xA8, 0x9F);
const WAVE_BLUE_2: Color = Color::Rgb(0x2D, 0x4F, 0x67);
const SURIMI_ORANGE: Color = Color::Rgb(0xFF, 0xA0, 0x66);

pub fn diff_add_bg() -> Style {
    Style::default().bg(DIFF_ADD_BG)
}

pub fn diff_remove_bg() -> Style {
    Style::default().bg(DIFF_DELETE_BG)
}

pub fn diff_add_marker() -> Style {
    Style::default().fg(SPRING_GREEN)
}

pub fn diff_remove_marker() -> Style {
    Style::default().fg(AUTUMN_RED)
}

pub fn hunk_header() -> Style {
    Style::default().fg(WAVE_AQUA_2).add_modifier(Modifier::BOLD)
}

pub fn file_header() -> Style {
    Style::default().fg(FUJI_WHITE).add_modifier(Modifier::BOLD)
}

pub fn file_header_collapsed() -> Style {
    Style::default().fg(FUJI_GRAY)
}

pub fn additions() -> Style {
    Style::default().fg(SPRING_GREEN)
}

pub fn deletions() -> Style {
    Style::default().fg(AUTUMN_RED)
}

pub fn comment_author() -> Style {
    Style::default().fg(AUTUMN_YELLOW).add_modifier(Modifier::BOLD)
}

pub fn comment_body() -> Style {
    Style::default().fg(FUJI_WHITE)
}

pub fn outdated() -> Style {
    Style::default().fg(KATANA_GRAY).add_modifier(Modifier::ITALIC)
}

pub fn resolved() -> Style {
    Style::default().fg(SUMI_INK_4)
}

pub fn description_header() -> Style {
    Style::default().fg(CRYSTAL_BLUE).add_modifier(Modifier::BOLD)
}

pub fn gutter() -> Style {
    Style::default().fg(SUMI_INK_4)
}

pub fn cursor_line() -> Style {
    Style::default().bg(WAVE_BLUE_2)
}

pub fn status_bar() -> Style {
    Style::default().bg(SUMI_INK_2).fg(FUJI_WHITE)
}

pub fn warning() -> Style {
    Style::default().fg(SURIMI_ORANGE).add_modifier(Modifier::BOLD)
}

pub fn modal_border() -> Style {
    Style::default().fg(CRYSTAL_BLUE)
}

pub fn modal_bg() -> Style {
    Style::default().bg(SUMI_INK_1)
}

pub fn thread_bar() -> Style {
    Style::default().fg(WAVE_AQUA_1)
}
