use crate::app::state::State;
use crate::diff::types::LineKind;

#[derive(Debug, Clone)]
pub enum Row {
    DescriptionHeader,
    DescriptionLine(String),
    FileHeader { file_idx: usize },
    HunkHeader { file_idx: usize, hunk_idx: usize },
    DiffLine { file_idx: usize, hunk_idx: usize, line_idx: usize },
    ThreadHeader { thread_idx: usize },
    ThreadComment { thread_idx: usize, comment_idx: usize },
    ThreadResolvedSummary { thread_idx: usize },
    DraftMarker { draft_idx: usize },
    Blank,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Anchor {
    Description,
    File(usize),
    Hunk(usize, usize),
    Line(usize, usize, usize),
    Thread(usize),
    Draft(usize),
}

pub fn build(state: &State) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();

    if let Some(overlay) = state.session.overlay() {
        if !overlay.description.trim().is_empty() {
            rows.push(Row::DescriptionHeader);
            if !state.fold.description_collapsed {
                for line in overlay.description.lines() {
                    rows.push(Row::DescriptionLine(line.to_string()));
                }
            }
            rows.push(Row::Blank);
        }
    }

    let diff = state.session.diff();
    for (fi, file) in diff.files.iter().enumerate() {
        rows.push(Row::FileHeader { file_idx: fi });

        if state.fold.is_collapsed(&file.path) {
            continue;
        }

        for (hi, hunk) in file.hunks.iter().enumerate() {
            rows.push(Row::HunkHeader {
                file_idx: fi,
                hunk_idx: hi,
            });

            for (li, line) in hunk.lines.iter().enumerate() {
                rows.push(Row::DiffLine {
                    file_idx: fi,
                    hunk_idx: hi,
                    line_idx: li,
                });

                if let Some(new_no) = line.new_lineno {
                    if line.kind != LineKind::Removed {
                        append_threads_for(&mut rows, state, &file.path, new_no);
                        append_drafts_for(&mut rows, state, &file.path, new_no);
                    }
                }
            }
        }
    }

    rows
}

fn append_drafts_for(rows: &mut Vec<Row>, state: &State, file: &str, line: u32) {
    for (di, c) in state.drafts.iter().enumerate() {
        if c.file == file && c.line == line {
            rows.push(Row::DraftMarker { draft_idx: di });
        }
    }
}

fn append_threads_for(rows: &mut Vec<Row>, state: &State, file: &str, line: u32) {
    let Some(overlay) = state.session.overlay() else {
        return;
    };
    for (ti, thread) in overlay.threads.iter().enumerate() {
        if thread.file != file || thread.line != line {
            continue;
        }
        if is_thread_collapsed(state, ti, thread) {
            rows.push(Row::ThreadResolvedSummary { thread_idx: ti });
            continue;
        }
        rows.push(Row::ThreadHeader { thread_idx: ti });
        for (ci, _) in thread.comments.iter().enumerate() {
            rows.push(Row::ThreadComment {
                thread_idx: ti,
                comment_idx: ci,
            });
        }
    }
}

pub fn is_thread_collapsed(
    state: &State,
    ti: usize,
    _thread: &crate::session::Thread,
) -> bool {
    !state.thread_overrides.contains(&ti)
}

pub fn anchor_of(row: &Row) -> Anchor {
    match row {
        Row::DescriptionHeader | Row::DescriptionLine(_) => Anchor::Description,
        Row::FileHeader { file_idx } => Anchor::File(*file_idx),
        Row::HunkHeader { file_idx, hunk_idx } => Anchor::Hunk(*file_idx, *hunk_idx),
        Row::DiffLine { file_idx, hunk_idx, line_idx } => {
            Anchor::Line(*file_idx, *hunk_idx, *line_idx)
        }
        Row::ThreadHeader { thread_idx }
        | Row::ThreadComment { thread_idx, .. }
        | Row::ThreadResolvedSummary { thread_idx } => Anchor::Thread(*thread_idx),
        Row::DraftMarker { draft_idx } => Anchor::Draft(*draft_idx),
        Row::Blank => Anchor::Description,
    }
}

pub fn current_draft_idx(rows: &[Row], cursor: usize) -> Option<usize> {
    match rows.get(cursor)? {
        Row::DraftMarker { draft_idx } => Some(*draft_idx),
        _ => None,
    }
}

pub fn find_anchor_index(rows: &[Row], anchor: &Anchor) -> Option<usize> {
    rows.iter().position(|r| &anchor_of(r) == anchor)
}

pub fn current_diff_line(state: &State, rows: &[Row], cursor: usize) -> Option<(String, u32)> {
    let row = rows.get(cursor)?;
    let (fi, hi, li) = match row {
        Row::DiffLine { file_idx, hunk_idx, line_idx } => (*file_idx, *hunk_idx, *line_idx),
        _ => return None,
    };
    let file = state.session.diff().files.get(fi)?;
    let line = file.hunks.get(hi)?.lines.get(li)?;
    let new_lineno = line.new_lineno?;
    Some((file.path.clone(), new_lineno))
}

pub fn current_thread_idx(rows: &[Row], cursor: usize) -> Option<usize> {
    match rows.get(cursor)? {
        Row::ThreadHeader { thread_idx }
        | Row::ThreadComment { thread_idx, .. }
        | Row::ThreadResolvedSummary { thread_idx } => Some(*thread_idx),
        _ => None,
    }
}
