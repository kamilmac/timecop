use crate::diff::types::{Diff, File, LineKind};
use crate::review::types::Draft;

pub fn format(draft: &Draft, diff: &Diff) -> String {
    let mut out = String::new();
    for c in &draft.new_comments {
        let Some(file) = diff.file(&c.file) else {
            continue;
        };
        let header = hunk_context(file, c.line);
        match header {
            Some(h) => out.push_str(&format!("{}:{}  ({h})\n", c.file, c.line)),
            None => out.push_str(&format!("{}:{}\n", c.file, c.line)),
        }
        for line in excerpt_around(file, c.line, 3) {
            out.push_str(&format!("> {line}\n"));
        }
        out.push_str(&format!("COMMENT: {}\n\n", c.body));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::types::{File, Hunk, Line, LineKind};
    use crate::review::types::{Draft, NewComment};

    fn fake_diff() -> Diff {
        Diff {
            files: vec![File {
                path: "src/app.rs".to_string(),
                hunks: vec![Hunk {
                    header: "@@ -10,5 +10,5 @@ fn handle_key".to_string(),
                    new_start: 10,
                    new_count: 5,
                    lines: vec![
                        Line { kind: LineKind::Context, old_lineno: Some(10), new_lineno: Some(10), text: "    let x = 1;".to_string() },
                        Line { kind: LineKind::Context, old_lineno: Some(11), new_lineno: Some(11), text: "    let y = 2;".to_string() },
                        Line { kind: LineKind::Removed, old_lineno: Some(12), new_lineno: None, text: "    foo()".to_string() },
                        Line { kind: LineKind::Added, old_lineno: None, new_lineno: Some(12), text: "    bar()".to_string() },
                        Line { kind: LineKind::Context, old_lineno: Some(13), new_lineno: Some(13), text: "    let z = 3;".to_string() },
                    ],
                }],
                additions: 1,
                deletions: 1,
            }],
        }
    }

    #[test]
    fn formats_comment_with_context() {
        let diff = fake_diff();
        let draft = Draft {
            new_comments: vec![NewComment {
                file: "src/app.rs".to_string(),
                line: 12,
                body: "use Result instead".to_string(),
            }],
            replies: vec![],
            resolutions: vec![],
            verdict: None,
        };
        let out = format(&draft, &diff);
        assert!(out.contains("src/app.rs:12"));
        assert!(out.contains("fn handle_key"));
        assert!(out.contains("COMMENT: use Result instead"));
        assert!(out.contains("> "));
    }
}
