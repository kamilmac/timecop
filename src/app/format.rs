use crate::diff::types::{Diff, File, LineKind};
use crate::session::Thread;

pub fn format_anchor(diff: &Diff, file: &str, line: u32) -> String {
    let Some(f) = diff.file(file) else {
        return format!("{file}:{line}\n");
    };
    let mut out = match hunk_context(f, line) {
        Some(h) => format!("{file}:{line}  ({h})\n"),
        None => format!("{file}:{line}\n"),
    };
    for excerpt in excerpt_around(f, line, 3) {
        out.push_str(&format!("> {excerpt}\n"));
    }
    out
}

pub fn format_thread(diff: &Diff, thread: &Thread) -> String {
    let mut out = format_anchor(diff, &thread.file, thread.line);
    for (i, c) in thread.comments.iter().enumerate() {
        if i == 0 {
            out.push_str(&format!("\n{} ({}):\n{}\n", c.author, c.created_at, c.body));
        } else {
            out.push_str(&format!("  └─ {}:\n  {}\n", c.author, c.body.replace('\n', "\n  ")));
        }
    }
    if thread.resolved {
        out.push_str("\n[resolved]\n");
    }
    if thread.outdated {
        out.push_str("\n[outdated]\n");
    }
    out
}

fn hunk_context(file: &File, line: u32) -> Option<String> {
    for hunk in &file.hunks {
        if line >= hunk.new_start && line < hunk.new_start + hunk.new_count {
            let after = hunk.header.splitn(3, "@@").nth(2)?.trim();
            if after.is_empty() {
                return None;
            }
            return Some(after.to_string());
        }
    }
    None
}

fn excerpt_around(file: &File, target_line: u32, context: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for hunk in &file.hunks {
        for line in &hunk.lines {
            if line.kind == LineKind::Removed {
                continue;
            }
            let Some(no) = line.new_lineno else { continue };
            if no + context >= target_line && no <= target_line {
                out.push(line.text.clone());
            }
        }
    }
    out
}
