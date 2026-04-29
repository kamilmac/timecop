use crate::app::state::{Bracket, State};
use crate::review::format;
use crate::review::types::{NewComment, Reply, Verdict};
use crate::session::Session;
use crate::session::pr as pr_session;
use crate::ui::input::{self, InputResult, InputState, InputTarget};
use crate::ui::items::{self, Anchor, Row};
use anyhow::Result;
use arboard::Clipboard;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key(state: &mut State, key: KeyEvent) -> Result<()> {
    if let Some(mut input) = state.input.take() {
        match input::handle_key(&mut input, key) {
            InputResult::Continue => {
                state.input = Some(input);
                return Ok(());
            }
            InputResult::Cancel => return Ok(()),
            InputResult::Submit(body) => {
                submit_input(state, input.target, body);
                return Ok(());
            }
        }
    }

    if state.show_verdict {
        return handle_verdict_key(state, key);
    }

    if state.show_help && !matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
        return Ok(());
    }

    handle_main_key(state, key)
}

fn handle_main_key(state: &mut State, key: KeyEvent) -> Result<()> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    if let Some(b) = state.pending_bracket {
        state.pending_bracket = None;
        let pred: fn(&Row) -> bool = match key.code {
            KeyCode::Char('f') => items::is_file_header,
            KeyCode::Char('c') => items::is_hunk_header,
            KeyCode::Char('r') => items::is_thread,
            _ => return Ok(()),
        };
        let rows = items::build(state);
        let target = match b {
            Bracket::Close => items::next_index(&rows, state.scroll.cursor, pred),
            Bracket::Open => items::prev_index(&rows, state.scroll.cursor, pred),
        };
        if let Some(idx) = target {
            state.scroll.jump_to(idx, rows.len());
        }
        return Ok(());
    }

    if state.pending_g {
        state.pending_g = false;
        if matches!(key.code, KeyCode::Char('g')) {
            let rows = items::build(state);
            state.scroll.jump_to(0, rows.len());
        }
        return Ok(());
    }

    match (key.code, ctrl) {
        (KeyCode::Char('q'), false) => state.quit = true,
        (KeyCode::Char('c'), true) => state.quit = true,
        (KeyCode::Char('?'), _) => state.show_help = !state.show_help,
        (KeyCode::Esc, _) => {
            state.show_help = false;
            state.show_verdict = false;
            state.status_message = None;
        }
        (KeyCode::Char('j'), false) | (KeyCode::Down, _) => move_line(state, 1),
        (KeyCode::Char('k'), false) | (KeyCode::Up, _) => move_line(state, -1),
        (KeyCode::Char('d'), true) => move_half_page(state, 1),
        (KeyCode::Char('u'), true) => move_half_page(state, -1),
        (KeyCode::Char('f'), true) => move_full_page(state, 1),
        (KeyCode::Char('b'), true) => move_full_page(state, -1),
        (KeyCode::Char('g'), false) => state.pending_g = true,
        (KeyCode::Char('G'), _) => {
            let rows = items::build(state);
            state.scroll.jump_to(rows.len().saturating_sub(1), rows.len());
        }
        (KeyCode::Char(']'), _) => state.pending_bracket = Some(Bracket::Close),
        (KeyCode::Char('['), _) => state.pending_bracket = Some(Bracket::Open),
        (KeyCode::Char(' '), _) => toggle_fold_at_cursor(state),
        (KeyCode::Char('z'), false) => toggle_all_folds(state),
        (KeyCode::Char('c'), false) => start_new_comment(state),
        (KeyCode::Char('r'), false) => start_reply(state),
        (KeyCode::Char('R'), _) => toggle_resolve(state),
        (KeyCode::Char('e'), false) => start_edit_draft(state),
        (KeyCode::Char('d'), false) => delete_draft_at_cursor(state),
        (KeyCode::Char('o'), false) => queue_editor_open(state),
        (KeyCode::Char('y'), false) => yank_to_clipboard(state)?,
        (KeyCode::Enter, _) => start_apply(state),
        _ => {}
    }
    Ok(())
}

fn handle_verdict_key(state: &mut State, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('a') => apply_review(state, Verdict::Approve)?,
        KeyCode::Char('x') => apply_review(state, Verdict::RequestChanges)?,
        KeyCode::Enter => apply_review(state, Verdict::Comment)?,
        KeyCode::Esc => state.show_verdict = false,
        _ => {}
    }
    Ok(())
}

fn move_line(state: &mut State, delta: isize) {
    let rows = items::build(state);
    state.scroll.move_by(delta, rows.len());
}

fn move_half_page(state: &mut State, dir: isize) {
    let rows = items::build(state);
    let h = 12isize;
    state.scroll.move_by(dir * h, rows.len());
}

fn move_full_page(state: &mut State, dir: isize) {
    let rows = items::build(state);
    let h = 24isize;
    state.scroll.move_by(dir * h, rows.len());
}

fn toggle_fold_at_cursor(state: &mut State) {
    let rows = items::build(state);
    let Some(row) = rows.get(state.scroll.cursor) else { return };
    let target = match items::anchor_of(row) {
        Anchor::File(fi) | Anchor::Hunk(fi, _) | Anchor::Line(fi, _, _) => state
            .session
            .diff()
            .files
            .get(fi)
            .map(|f| f.path.clone()),
        Anchor::Description => {
            state.fold.description_collapsed = !state.fold.description_collapsed;
            return;
        }
        _ => None,
    };
    if let Some(path) = target {
        state.fold.toggle(&path);
        let new_rows = items::build(state);
        if let Some(idx) = items::find_anchor_index(
            &new_rows,
            &Anchor::File(
                state
                    .session
                    .diff()
                    .files
                    .iter()
                    .position(|f| f.path == path)
                    .unwrap_or(0),
            ),
        ) {
            state.scroll.jump_to(idx, new_rows.len());
        }
    }
}

fn toggle_all_folds(state: &mut State) {
    let paths = state.all_paths();
    if state.fold.any_expanded(&paths) {
        state.fold.collapse_all(&paths);
    } else {
        state.fold.expand_all();
    }
    let rows = items::build(state);
    if state.scroll.cursor >= rows.len() {
        state.scroll.cursor = rows.len().saturating_sub(1);
    }
}

fn start_new_comment(state: &mut State) {
    let rows = items::build(state);
    if let Some((file, line)) = items::current_diff_line(state, &rows, state.scroll.cursor) {
        state.input = Some(InputState::new(InputTarget::NewComment { file, line }));
    } else {
        state.status_message = Some("place cursor on a code line".to_string());
    }
}

fn start_reply(state: &mut State) {
    let rows = items::build(state);
    let Some(ti) = items::current_thread_idx(&rows, state.scroll.cursor) else {
        state.status_message = Some("place cursor on a thread".to_string());
        return;
    };
    let Some(overlay) = state.session.overlay() else { return };
    let Some(thread) = overlay.threads.get(ti) else { return };
    if thread.outdated {
        state.status_message = Some("can't reply to outdated thread".to_string());
        return;
    }
    state.input = Some(InputState::new(InputTarget::Reply {
        thread_id: thread.node_id.clone(),
    }));
}

fn toggle_resolve(state: &mut State) {
    let rows = items::build(state);
    let Some(ti) = items::current_thread_idx(&rows, state.scroll.cursor) else { return };
    let Some(overlay) = state.session.overlay() else { return };
    let Some(thread) = overlay.threads.get(ti) else { return };
    let id = thread.node_id.clone();
    if let Some(pos) = state.draft.resolutions.iter().position(|t| t == &id) {
        state.draft.resolutions.remove(pos);
    } else {
        state.draft.resolutions.push(id);
    }
}

fn start_edit_draft(state: &mut State) {
    let rows = items::build(state);
    let Some(di) = items::current_draft_idx(&rows, state.scroll.cursor) else {
        state.status_message = Some("place cursor on a draft".to_string());
        return;
    };
    let Some(c) = state.draft.new_comments.get(di) else { return };
    state.input = Some(InputState::with_body(
        InputTarget::EditDraft { draft_idx: di },
        c.body.clone(),
    ));
}

fn delete_draft_at_cursor(state: &mut State) {
    let rows = items::build(state);
    let Some(di) = items::current_draft_idx(&rows, state.scroll.cursor) else {
        state.status_message = Some("place cursor on a draft".to_string());
        return;
    };
    if di < state.draft.new_comments.len() {
        state.draft.new_comments.remove(di);
    }
}

fn queue_editor_open(state: &mut State) {
    let rows = items::build(state);
    let row = rows.get(state.scroll.cursor);
    let target = match row.map(items::anchor_of) {
        Some(Anchor::File(fi)) => state
            .session
            .diff()
            .files
            .get(fi)
            .map(|f| (f.path.clone(), 1u32)),
        Some(Anchor::Hunk(fi, hi)) => state
            .session
            .diff()
            .files
            .get(fi)
            .and_then(|f| f.hunks.get(hi).map(|h| (f.path.clone(), h.new_start))),
        Some(Anchor::Line(fi, hi, li)) => {
            let f = state.session.diff().files.get(fi);
            f.and_then(|f| {
                f.hunks.get(hi).and_then(|h| {
                    let line = h.lines.get(li)?;
                    let lineno = line
                        .new_lineno
                        .or_else(|| {
                            h.lines
                                .iter()
                                .skip(li)
                                .find_map(|l| l.new_lineno)
                        })
                        .unwrap_or(h.new_start);
                    Some((f.path.clone(), lineno))
                })
            })
        }
        Some(Anchor::Thread(ti)) => state
            .session
            .overlay()
            .and_then(|o| o.threads.get(ti).map(|t| (t.file.clone(), t.line))),
        Some(Anchor::Draft(di)) => state
            .draft
            .new_comments
            .get(di)
            .map(|c| (c.file.clone(), c.line)),
        _ => None,
    };
    state.pending_editor = target;
}

fn yank_to_clipboard(state: &mut State) -> Result<()> {
    if matches!(state.session, Session::Pr(_)) {
        state.status_message = Some("PR mode — use Enter to apply".to_string());
        return Ok(());
    }
    if state.draft.is_empty() {
        state.status_message = Some("no comments to copy".to_string());
        return Ok(());
    }
    let text = format::format(&state.draft, state.session.diff());
    let mut clip = Clipboard::new()?;
    clip.set_text(text)?;
    state.status_message = Some(format!(
        "copied {} comment(s) to clipboard",
        state.draft.new_comments.len()
    ));
    Ok(())
}

fn start_apply(state: &mut State) {
    if !matches!(state.session, Session::Pr(_)) {
        state.status_message = Some("not a PR — use y to copy".to_string());
        return;
    }
    if state.draft.is_empty() {
        state.status_message = Some("no draft to apply".to_string());
        return;
    }
    state.show_verdict = true;
}

fn apply_review(state: &mut State, verdict: Verdict) -> Result<()> {
    state.show_verdict = false;
    state.draft.verdict = Some(verdict);
    let session = match &state.session {
        Session::Pr(p) => p,
        _ => return Ok(()),
    };
    match pr_session::apply(session, &state.draft) {
        Ok(()) => {
            state.draft = Default::default();
            state.status_message = Some("review submitted".to_string());
        }
        Err(e) => {
            state.status_message = Some(format!("apply failed: {e}"));
        }
    }
    Ok(())
}

fn submit_input(state: &mut State, target: InputTarget, body: String) {
    let body = body.trim().to_string();
    if body.is_empty() {
        return;
    }
    match target {
        InputTarget::NewComment { file, line } => {
            state.draft.new_comments.push(NewComment { file, line, body });
        }
        InputTarget::Reply { thread_id } => {
            state.draft.replies.push(Reply { thread_id, body });
        }
        InputTarget::EditDraft { draft_idx } => {
            if let Some(c) = state.draft.new_comments.get_mut(draft_idx) {
                c.body = body;
            }
        }
    }
}
