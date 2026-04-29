use crate::app::state::State;
use crate::session::Session;
use crate::ui::theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn render(state: &State, frame: &mut Frame, area: Rect) {
    let mut spans: Vec<Span<'static>> = Vec::new();

    match &state.session {
        Session::Pr(p) => {
            spans.push(Span::raw(format!("PR #{} · ", p.number)));
            spans.push(Span::raw(format!("{} → {} · ", p.head_ref, p.base_ref)));
        }
        Session::Branch(b) => {
            spans.push(Span::raw(format!("{} → {} ", b.head_ref, b.base_ref)));
            spans.push(Span::raw(format!("(merge-base {}) · ", b.merge_base_short)));
        }
    }

    let n_files = state.session.diff().files.len();
    spans.push(Span::raw(format!(
        "{n_files} file{}",
        if n_files == 1 { "" } else { "s" }
    )));

    if let Some(msg) = &state.status_message {
        spans.push(Span::raw(" · "));
        spans.push(Span::styled(msg.clone(), theme::warning()));
    }

    let line = Line::from(spans).style(theme::status_bar());
    frame.render_widget(Paragraph::new(line), area);
}
