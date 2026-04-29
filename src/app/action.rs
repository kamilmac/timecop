use crate::app::format;
use crate::app::state::State;
use crate::session::{Session, Verdict};
use crate::session::branch as branch_session;
use crate::session::pr as pr_session;
use crate::ui::input::{self, InputResult, InputState, InputTarget};
use crate::ui::items::{self, Anchor};
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
                return submit_input(state, input.target, body);
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
        (KeyCode::Char('J'), _) => move_line(state, 5),
        (KeyCode::Char('K'), _) => move_line(state, -5),
        (KeyCode::Char('f'), true) => move_full_page(state, 1),
        (KeyCode::Char('b'), true) => move_full_page(state, -1),
        (KeyCode::Char('g'), false) => state.pending_g = true,
        (KeyCode::Char('G'), _) => {
            let rows = items::build(state);
            state.scroll.jump_to(rows.len().saturating_sub(1), rows.len());
        }
        (KeyCode::Char(' '), _) => toggle_fold_at_cursor(state),
        (KeyCode::Char('h'), false) => collapse_at_cursor(state),
        (KeyCode::Char('l'), false) => expand_at_cursor(state),
        (KeyCode::Char('z'), false) => toggle_all_folds(state),
        (KeyCode::Char('c'), false) => start_comment(state),
        (KeyCode::Char('R'), _) => toggle_resolve(state)?,
        (KeyCode::Char('o'), false) => queue_editor_open(state),
        (KeyCode::Char('r'), false) => refresh(state)?,
        (KeyCode::Char('+'), _) => react_at_cursor(state, "+1")?,
        (KeyCode::Char('y'), false) => yank_at_cursor(state)?,
        (KeyCode::Enter, _) => start_verdict(state),
        _ => {}
    }
    Ok(())
}

fn handle_verdict_key(state: &mut State, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('a') => submit_verdict(state, Verdict::Approve)?,
        KeyCode::Char('x') => submit_verdict(state, Verdict::RequestChanges)?,
        KeyCode::Esc => state.show_verdict = false,
        _ => {}
    }
    Ok(())
}

fn move_line(state: &mut State, delta: isize) {
    let rows = items::build(state);
    state.scroll.move_by(delta, rows.len());
}

fn move_full_page(state: &mut State, dir: isize) {
    let rows = items::build(state);
    let h = 24isize;
    state.scroll.move_by(dir * h, rows.len());
}

fn toggle_fold_at_cursor(state: &mut State) {
    let rows = items::build(state);
    let Some(row) = rows.get(state.scroll.cursor) else { return };
    match items::anchor_of(row) {
        Anchor::File(fi) | Anchor::Hunk(fi, _) | Anchor::Line(fi, _, _) => {
            let Some(path) = state.session.diff().files.get(fi).map(|f| f.path.clone()) else {
                return;
            };
            state.fold.toggle(&path);
            let new_rows = items::build(state);
            if let Some(idx) = items::find_anchor_index(&new_rows, &Anchor::File(fi)) {
                state.scroll.jump_to(idx, new_rows.len());
            }
        }
        Anchor::Thread(ti) => {
            toggle_thread(state, ti);
        }
        Anchor::Description => {
            state.fold.description_collapsed = !state.fold.description_collapsed;
        }
    }
}

fn toggle_thread(state: &mut State, ti: usize) {
    if state.thread_overrides.contains(&ti) {
        state.thread_overrides.remove(&ti);
    } else {
        state.thread_overrides.insert(ti);
    }
}

fn collapse_at_cursor(state: &mut State) {
    let rows = items::build(state);
    let Some(row) = rows.get(state.scroll.cursor) else { return };
    match items::anchor_of(row) {
        Anchor::File(_) | Anchor::Hunk(_, _) | Anchor::Line(_, _, _) => {
            if let Some(path) = file_path_at_cursor(state) {
                if !state.fold.is_collapsed(&path) {
                    toggle_fold_at_cursor(state);
                }
            }
        }
        Anchor::Thread(ti) => {
            if let Some(thread) = state.session.overlay().and_then(|o| o.threads.get(ti)) {
                if !items::is_thread_collapsed(state, ti, thread) {
                    toggle_thread(state, ti);
                }
            }
        }
        Anchor::Description => {
            state.fold.description_collapsed = true;
        }
    }
}

fn expand_at_cursor(state: &mut State) {
    let rows = items::build(state);
    let Some(row) = rows.get(state.scroll.cursor) else { return };
    match items::anchor_of(row) {
        Anchor::File(_) | Anchor::Hunk(_, _) | Anchor::Line(_, _, _) => {
            if let Some(path) = file_path_at_cursor(state) {
                if state.fold.is_collapsed(&path) {
                    toggle_fold_at_cursor(state);
                }
            }
        }
        Anchor::Thread(ti) => {
            if let Some(thread) = state.session.overlay().and_then(|o| o.threads.get(ti)) {
                if items::is_thread_collapsed(state, ti, thread) {
                    toggle_thread(state, ti);
                }
            }
        }
        Anchor::Description => {
            state.fold.description_collapsed = false;
        }
    }
}

fn file_path_at_cursor(state: &State) -> Option<String> {
    let rows = items::build(state);
    let row = rows.get(state.scroll.cursor)?;
    match items::anchor_of(row) {
        Anchor::File(fi) | Anchor::Hunk(fi, _) | Anchor::Line(fi, _, _) => state
            .session
            .diff()
            .files
            .get(fi)
            .map(|f| f.path.clone()),
        _ => None,
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

fn start_comment(state: &mut State) {
    if !matches!(state.session, Session::Pr(_)) {
        state.status_message = Some("branch mode is read-only — y to yank".to_string());
        return;
    }
    let rows = items::build(state);

    if let Some(ti) = items::current_thread_idx(&rows, state.scroll.cursor) {
        let Some(overlay) = state.session.overlay() else { return };
        let Some(thread) = overlay.threads.get(ti) else { return };
        if thread.outdated {
            state.status_message = Some("can't reply to outdated thread".to_string());
            return;
        }
        state.input = Some(InputState::new(InputTarget::Reply {
            thread_id: thread.node_id.clone(),
        }));
        return;
    }

    if let Some((file, line)) = items::current_diff_line(state, &rows, state.scroll.cursor) {
        state.input = Some(InputState::new(InputTarget::NewComment { file, line }));
    } else {
        state.status_message = Some("place cursor on a code line or thread".to_string());
    }
}

fn toggle_resolve(state: &mut State) -> Result<()> {
    let rows = items::build(state);
    let Some(ti) = items::current_thread_idx(&rows, state.scroll.cursor) else { return Ok(()); };
    let (node_id, target) = match &state.session {
        Session::Pr(p) => match p.overlay.threads.get(ti) {
            Some(t) => (t.node_id.clone(), !t.resolved),
            None => return Ok(()),
        },
        _ => {
            state.status_message = Some("PR mode only".to_string());
            return Ok(());
        }
    };
    match pr_session::set_resolved(&node_id, target) {
        Ok(()) => {
            if let Session::Pr(p) = &mut state.session {
                if let Some(t) = p.overlay.threads.get_mut(ti) {
                    t.resolved = target;
                }
            }
            state.status_message = Some(
                if target { "resolved" } else { "unresolved" }.to_string(),
            );
        }
        Err(e) => {
            state.status_message = Some(format!("resolve failed: {e}"));
        }
    }
    Ok(())
}

fn queue_editor_open(state: &mut State) {
    let rows = items::build(state);
    let row = rows.get(state.scroll.cursor);
    let target = match row.map(items::anchor_of) {
        Some(Anchor::File(fi)) => state.session.diff().files.get(fi).map(|f| (f.path.clone(), 1u32)),
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
                        .or_else(|| h.lines.iter().skip(li).find_map(|l| l.new_lineno))
                        .unwrap_or(h.new_start);
                    Some((f.path.clone(), lineno))
                })
            })
        }
        Some(Anchor::Thread(ti)) => state
            .session
            .overlay()
            .and_then(|o| o.threads.get(ti).map(|t| (t.file.clone(), t.line))),
        _ => None,
    };
    state.pending_editor = target;
}

fn yank_at_cursor(state: &mut State) -> Result<()> {
    let rows = items::build(state);
    let Some(row) = rows.get(state.scroll.cursor) else {
        return Ok(());
    };
    let payload: Option<String> = match items::anchor_of(row) {
        Anchor::Thread(ti) => state
            .session
            .overlay()
            .and_then(|o| o.threads.get(ti))
            .map(|t| format::format_thread(state.session.diff(), t)),
        Anchor::Line(fi, hi, li) => {
            let diff = state.session.diff();
            let f = diff.files.get(fi);
            let line = f.and_then(|f| f.hunks.get(hi)).and_then(|h| h.lines.get(li));
            match (f, line.and_then(|l| l.new_lineno)) {
                (Some(f), Some(n)) => Some(format::format_anchor(diff, &f.path, n)),
                _ => None,
            }
        }
        _ => None,
    };
    let Some(payload) = payload else {
        state.status_message = Some("nothing to copy here".to_string());
        return Ok(());
    };
    let mut clip = Clipboard::new()?;
    clip.set_text(format!("{}{payload}", yank_header(&state.session)))?;
    state.status_message = Some("copied to clipboard".to_string());
    Ok(())
}

fn yank_header(session: &Session) -> String {
    match session {
        Session::Branch(b) => format!("Branch: {} (vs {})\n\n", b.head_ref, b.base_ref),
        Session::Pr(p) => format!("PR #{}: {} → {}\n\n", p.number, p.head_ref, p.base_ref),
    }
}

fn react_at_cursor(state: &mut State, content: &str) -> Result<()> {
    let rows = items::build(state);
    let Some(row) = rows.get(state.scroll.cursor) else { return Ok(()); };
    let (ti, ci) = match row {
        items::Row::ThreadHeader { thread_idx }
        | items::Row::ThreadResolvedSummary { thread_idx } => (*thread_idx, 0usize),
        items::Row::ThreadComment { thread_idx, comment_idx } => (*thread_idx, *comment_idx),
        _ => {
            state.status_message = Some("place cursor on a comment".to_string());
            return Ok(());
        }
    };
    let comment_id = match &state.session {
        Session::Pr(p) => p
            .overlay
            .threads
            .get(ti)
            .and_then(|t| t.comments.get(ci))
            .map(|c| c.id),
        _ => {
            state.status_message = Some("reactions only work on PR comments".to_string());
            return Ok(());
        }
    };
    let Some(cid) = comment_id else { return Ok(()); };
    let result = match &state.session {
        Session::Pr(p) => pr_session::post_reaction(p, cid, content),
        _ => return Ok(()),
    };
    match result {
        Ok(()) => {
            if let Session::Pr(p) = &mut state.session {
                if let Some(c) = p
                    .overlay
                    .threads
                    .get_mut(ti)
                    .and_then(|t| t.comments.get_mut(ci))
                {
                    match content {
                        "+1" => c.reactions.thumbs_up += 1,
                        "-1" => c.reactions.thumbs_down += 1,
                        "heart" => c.reactions.heart += 1,
                        "hooray" => c.reactions.hooray += 1,
                        "laugh" => c.reactions.laugh += 1,
                        "confused" => c.reactions.confused += 1,
                        "rocket" => c.reactions.rocket += 1,
                        "eyes" => c.reactions.eyes += 1,
                        _ => {}
                    }
                }
            }
            state.status_message = Some(format!("reacted {content}"));
        }
        Err(e) => {
            state.status_message = Some(format!("react failed: {e}"));
        }
    }
    Ok(())
}

fn refresh(state: &mut State) -> Result<()> {
    let new_session = match &state.session {
        Session::Branch(b) => Session::Branch(branch_session::open(
            state.repo_path.clone(),
            Some(b.head_ref.clone()),
        )?),
        Session::Pr(p) => Session::Pr(pr_session::open(p.number)?),
    };
    state.session = new_session;
    state.thread_overrides.clear();
    let rows = items::build(state);
    if state.scroll.cursor >= rows.len() {
        state.scroll.cursor = rows.len().saturating_sub(1);
    }
    state.status_message = Some("refreshed".to_string());
    Ok(())
}

fn start_verdict(state: &mut State) {
    if !matches!(state.session, Session::Pr(_)) {
        state.status_message = Some("not a PR".to_string());
        return;
    }
    state.show_verdict = true;
}

fn submit_verdict(state: &mut State, verdict: Verdict) -> Result<()> {
    state.show_verdict = false;
    let session = match &state.session {
        Session::Pr(p) => p,
        _ => return Ok(()),
    };
    match pr_session::submit_verdict(session, verdict) {
        Ok(()) => {
            state.status_message = Some(match verdict {
                Verdict::Approve => "approved".to_string(),
                Verdict::RequestChanges => "requested changes".to_string(),
            });
        }
        Err(e) => {
            state.status_message = Some(format!("submit failed: {e}"));
        }
    }
    Ok(())
}

fn submit_input(state: &mut State, target: InputTarget, body: String) -> Result<()> {
    let body = body.trim().to_string();
    if body.is_empty() {
        return Ok(());
    }
    let Session::Pr(_) = &state.session else {
        return Ok(());
    };

    match target {
        InputTarget::NewComment { file, line } => {
            let result = match &state.session {
                Session::Pr(p) => pr_session::post_comment(p, &file, line, &body),
                _ => unreachable!(),
            };
            match result {
                Ok(()) => {
                    refresh(state)?;
                    state.status_message = Some("comment posted".to_string());
                }
                Err(e) => {
                    state.status_message = Some(format!("post failed: {e}"));
                }
            }
        }
        InputTarget::Reply { thread_id } => {
            let root_id = match &state.session {
                Session::Pr(p) => p
                    .overlay
                    .threads
                    .iter()
                    .find(|t| t.node_id == thread_id)
                    .map(|t| t.root_comment_id),
                _ => None,
            };
            let Some(rid) = root_id else {
                state.status_message = Some("reply target not found".to_string());
                return Ok(());
            };
            let result = match &state.session {
                Session::Pr(p) => pr_session::post_reply(p, rid, &body),
                _ => unreachable!(),
            };
            match result {
                Ok(()) => {
                    refresh(state)?;
                    state.status_message = Some("reply posted".to_string());
                }
                Err(e) => {
                    state.status_message = Some(format!("reply failed: {e}"));
                }
            }
        }
    }
    Ok(())
}
