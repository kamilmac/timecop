use crate::app::state::State;
use crate::diff::types::LineKind;
use crate::ui::input::{InputState, InputTarget};
use crate::ui::items::{self, Row};
use crate::ui::theme;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

pub fn render(state: &mut State, frame: &mut Frame) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let main_area = chunks[0];
    let status_area = chunks[1];

    let rows = items::build(state);
    state
        .scroll
        .ensure_visible(main_area.height as usize, rows.len());

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(main_area.height as usize);
    let viewport = main_area.height as usize;
    for i in 0..viewport {
        let idx = state.scroll.offset + i;
        let Some(row) = rows.get(idx) else { break };
        let mut line = render_row(state, row);
        if idx == state.scroll.cursor {
            line.style = line.style.patch(theme::cursor_line());
        }
        lines.push(line);
    }

    frame.render_widget(Paragraph::new(lines), main_area);
    crate::ui::status::render(state, frame, status_area);

    if state.show_help {
        crate::ui::help::render(frame, area);
    }

    if let Some(input) = state.input.clone() {
        render_input(&input, frame, area);
    }

    if state.show_verdict {
        render_verdict(frame, area);
    }
}

fn render_row(state: &State, row: &Row) -> Line<'static> {
    match row {
        Row::DescriptionHeader => {
            let chevron = if state.fold.description_collapsed { "▸" } else { "▾" };
            Line::from(vec![
                Span::styled(format!("{chevron} "), theme::file_header_collapsed()),
                Span::styled("Description".to_string(), theme::description_header()),
            ])
        }
        Row::DescriptionLine(s) => Line::from(vec![Span::raw(format!("  {s}"))]),
        Row::Blank => Line::from(""),
        Row::FileHeader { file_idx } => render_file_header(state, *file_idx),
        Row::HunkHeader { file_idx, hunk_idx } => render_hunk_header(state, *file_idx, *hunk_idx),
        Row::DiffLine { file_idx, hunk_idx, line_idx } => {
            render_diff_line(state, *file_idx, *hunk_idx, *line_idx)
        }
        Row::ThreadHeader { thread_idx } => render_thread_header(state, *thread_idx),
        Row::ThreadComment { thread_idx, comment_idx } => {
            render_thread_comment(state, *thread_idx, *comment_idx)
        }
        Row::ThreadResolvedSummary { thread_idx } => render_thread_summary(state, *thread_idx),
    }
}

fn render_file_header(state: &State, fi: usize) -> Line<'static> {
    let Some(file) = state.session.diff().files.get(fi) else {
        return Line::from("");
    };
    let collapsed = state.fold.is_collapsed(&file.path);
    let chevron = if collapsed { "▸" } else { "▾" };
    let path_style = if collapsed {
        theme::file_header_collapsed()
    } else {
        theme::file_header()
    };
    let mut spans = vec![
        Span::styled(format!("{chevron} "), theme::file_header_collapsed()),
        Span::styled(file.path.clone(), path_style),
        Span::raw("  "),
        Span::styled(format!("[+{}", file.additions), theme::additions()),
        Span::raw(" "),
        Span::styled(format!("-{}]", file.deletions), theme::deletions()),
    ];

    let thread_count = state
        .session
        .overlay()
        .map(|o| {
            o.threads
                .iter()
                .filter(|t| t.file == file.path && !t.outdated)
                .count()
        })
        .unwrap_or(0);
    if thread_count > 0 {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("{thread_count} thread{}", if thread_count == 1 { "" } else { "s" }),
            theme::comment_author(),
        ));
    }

    Line::from(spans)
}

fn render_hunk_header(state: &State, fi: usize, hi: usize) -> Line<'static> {
    let Some(file) = state.session.diff().files.get(fi) else {
        return Line::from("");
    };
    let Some(hunk) = file.hunks.get(hi) else {
        return Line::from("");
    };
    Line::from(vec![
        Span::raw("    ".to_string()),
        Span::styled(hunk.header.clone(), theme::hunk_header()),
    ])
}

fn render_diff_line(state: &State, fi: usize, hi: usize, li: usize) -> Line<'static> {
    let Some(file) = state.session.diff().files.get(fi) else {
        return Line::from("");
    };
    let Some(hunk) = file.hunks.get(hi) else {
        return Line::from("");
    };
    let Some(line) = hunk.lines.get(li) else {
        return Line::from("");
    };

    let old_no = line
        .old_lineno
        .map(|n| format!("{n:>4}"))
        .unwrap_or_else(|| "    ".to_string());
    let new_no = line
        .new_lineno
        .map(|n| format!("{n:>4}"))
        .unwrap_or_else(|| "    ".to_string());

    let (marker, marker_style, line_bg) = match line.kind {
        LineKind::Added => ("+ ", theme::diff_add_marker(), Some(theme::diff_add_bg())),
        LineKind::Removed => ("- ", theme::diff_remove_marker(), Some(theme::diff_remove_bg())),
        LineKind::Context => ("  ", theme::gutter(), None),
    };

    let mut spans = vec![
        Span::styled(format!(" {old_no} "), theme::gutter()),
        Span::styled(format!("{new_no} │"), theme::gutter()),
        Span::styled(marker.to_string(), marker_style),
    ];

    let highlighted = state.highlighter.highlight(&file.path, &line.text);
    spans.extend(highlighted);

    let mut out = Line::from(spans);
    if let Some(bg) = line_bg {
        out.style = bg;
    }
    out
}

fn render_thread_header(state: &State, ti: usize) -> Line<'static> {
    let Some(thread) = state.session.overlay().and_then(|o| o.threads.get(ti)) else {
        return Line::from("");
    };
    let Some(first) = thread.comments.first() else {
        return Line::from("");
    };
    let age = relative_age(&first.created_at);
    Line::from(vec![
        Span::raw("        ".to_string()),
        Span::styled("┃ ".to_string(), theme::thread_bar()),
        Span::styled(first.author.clone(), theme::comment_author()),
        Span::raw(" · "),
        Span::styled(age, theme::resolved()),
    ])
}

fn render_thread_comment(state: &State, ti: usize, ci: usize) -> Line<'static> {
    let Some(thread) = state.session.overlay().and_then(|o| o.threads.get(ti)) else {
        return Line::from("");
    };
    let Some(comment) = thread.comments.get(ci) else {
        return Line::from("");
    };
    let prefix = if ci == 0 { "  " } else { "└─ " };
    let body = first_line_of(&comment.body);
    let mut spans = vec![
        Span::raw("        ".to_string()),
        Span::styled("┃ ".to_string(), theme::thread_bar()),
        Span::raw(prefix.to_string()),
    ];
    if ci > 0 {
        spans.push(Span::styled(comment.author.clone(), theme::comment_author()));
        spans.push(Span::raw(": "));
    }
    spans.push(Span::styled(body, theme::comment_body()));
    append_reactions(&mut spans, &comment.reactions);
    Line::from(spans)
}

fn append_reactions(spans: &mut Vec<Span<'static>>, r: &crate::session::Reactions) {
    if r.is_empty() {
        return;
    }
    let pairs = [
        ("👍", r.thumbs_up),
        ("👎", r.thumbs_down),
        ("❤️", r.heart),
        ("🎉", r.hooray),
        ("😄", r.laugh),
        ("😕", r.confused),
        ("🚀", r.rocket),
        ("👀", r.eyes),
    ];
    for (emoji, count) in pairs {
        if count > 0 {
            spans.push(Span::raw("  ".to_string()));
            spans.push(Span::styled(
                format!("{emoji} {count}"),
                theme::resolved(),
            ));
        }
    }
}

fn render_thread_summary(state: &State, ti: usize) -> Line<'static> {
    let Some(thread) = state.session.overlay().and_then(|o| o.threads.get(ti)) else {
        return Line::from("");
    };
    let Some(first) = thread.comments.first() else {
        return Line::from("");
    };
    let body_style = if thread.resolved || thread.outdated {
        theme::resolved()
    } else {
        theme::comment_body()
    };

    let mut spans = vec![
        Span::raw("        ".to_string()),
        Span::styled("┃ ".to_string(), theme::thread_bar()),
        Span::styled(first.author.clone(), theme::comment_author()),
        Span::raw(" · "),
        Span::styled(relative_age(&first.created_at), theme::resolved()),
        Span::raw(": "),
        Span::styled(first_line_of(&first.body), body_style),
    ];

    let extra = thread.comments.len().saturating_sub(1);
    if extra > 0 {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(
            format!("(+{extra})"),
            theme::resolved(),
        ));
    }
    if thread.resolved {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[resolved]".to_string(), theme::resolved()));
    }
    if thread.outdated {
        spans.push(Span::raw(" "));
        spans.push(Span::styled("[outdated]".to_string(), theme::outdated()));
    }

    Line::from(spans)
}

fn first_line_of(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

fn relative_age(ts: &str) -> String {
    if ts.len() >= 10 {
        ts[..10].to_string()
    } else {
        ts.to_string()
    }
}

fn render_input(input: &InputState, frame: &mut Frame, area: Rect) {
    let modal = centered_rect(70, 40, area);
    frame.render_widget(Clear, modal);
    let title = match &input.target {
        InputTarget::NewComment { file, line } => format!(" New comment · {file}:{line} "),
        InputTarget::Reply { .. } => " Reply ".to_string(),
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme::modal_border())
        .style(theme::modal_bg());
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let content = format!("{}\n\n[Ctrl+S submit · Esc cancel]", input.buffer);
    let para = Paragraph::new(content)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White));
    frame.render_widget(para, inner);
}

fn render_verdict(frame: &mut Frame, area: Rect) {
    let modal = centered_rect(40, 30, area);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .title(" Submit review ")
        .borders(Borders::ALL)
        .border_style(theme::modal_border())
        .style(theme::modal_bg());
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" a ", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw("  Approve"),
        ]),
        Line::from(vec![
            Span::styled(" x ", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw("  Request changes"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Esc ", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw("  Cancel"),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
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
