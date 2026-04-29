use crate::diff::types::{Diff, File, Hunk, Line, LineKind};
use anyhow::{Context, Result, anyhow};

pub fn parse(unified: &str) -> Result<Diff> {
    let mut files: Vec<File> = Vec::new();
    let mut current_file: Option<File> = None;
    let mut current_hunk: Option<Hunk> = None;
    let mut old_lineno: u32 = 0;
    let mut new_lineno: u32 = 0;

    for raw in unified.lines() {
        if let Some(rest) = raw.strip_prefix("diff --git ") {
            if let Some(file) = current_file.take() {
                files.push(finalize_file(file, current_hunk.take()));
            }
            let path = parse_diff_header_path(rest)?;
            current_file = Some(File {
                path,
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
            });
            continue;
        }

        if raw.starts_with("--- ") || raw.starts_with("+++ ") || raw.starts_with("index ")
            || raw.starts_with("similarity ") || raw.starts_with("rename ")
            || raw.starts_with("new file ") || raw.starts_with("deleted file ")
            || raw.starts_with("Binary ") || raw.starts_with("old mode ")
            || raw.starts_with("new mode ") || raw.starts_with("copy ")
            || raw.starts_with("dissimilarity ")
        {
            continue;
        }

        if raw.starts_with("@@") {
            if let Some(hunk) = current_hunk.take() {
                if let Some(file) = current_file.as_mut() {
                    file.hunks.push(hunk);
                }
            }
            let (old_start, _old_count, new_start, new_count) = parse_hunk_header(raw)?;
            old_lineno = old_start;
            new_lineno = new_start;
            current_hunk = Some(Hunk {
                header: raw.to_string(),
                new_start,
                new_count,
                lines: Vec::new(),
            });
            continue;
        }

        if current_hunk.is_none() {
            continue;
        }

        let hunk = current_hunk.as_mut().unwrap();
        let file = current_file.as_mut().unwrap();

        let (kind, text, old_no, new_no) = if let Some(t) = raw.strip_prefix('+') {
            file.additions += 1;
            let no = new_lineno;
            new_lineno += 1;
            (LineKind::Added, t.to_string(), None, Some(no))
        } else if let Some(t) = raw.strip_prefix('-') {
            file.deletions += 1;
            let no = old_lineno;
            old_lineno += 1;
            (LineKind::Removed, t.to_string(), Some(no), None)
        } else if let Some(t) = raw.strip_prefix(' ') {
            let o = old_lineno;
            let n = new_lineno;
            old_lineno += 1;
            new_lineno += 1;
            (LineKind::Context, t.to_string(), Some(o), Some(n))
        } else if raw.starts_with('\\') {
            continue;
        } else {
            continue;
        };

        hunk.lines.push(Line {
            kind,
            old_lineno: old_no,
            new_lineno: new_no,
            text,
        });
    }

    if let Some(file) = current_file.take() {
        files.push(finalize_file(file, current_hunk.take()));
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Diff { files })
}

fn finalize_file(mut file: File, last_hunk: Option<Hunk>) -> File {
    if let Some(h) = last_hunk {
        file.hunks.push(h);
    }
    file
}

fn parse_diff_header_path(rest: &str) -> Result<String> {
    let parts: Vec<&str> = rest.split_whitespace().collect();
    let last = parts
        .last()
        .ok_or_else(|| anyhow!("malformed diff header: {rest}"))?;
    let path = last
        .strip_prefix("b/")
        .or_else(|| last.strip_prefix("\"b/"))
        .unwrap_or(last);
    let path = path.trim_end_matches('"');
    Ok(path.to_string())
}

fn parse_hunk_header(line: &str) -> Result<(u32, u32, u32, u32)> {
    let inner = line
        .strip_prefix("@@ ")
        .and_then(|s| s.split(" @@").next())
        .ok_or_else(|| anyhow!("malformed hunk header: {line}"))?;
    let parts: Vec<&str> = inner.split_whitespace().collect();
    if parts.len() < 2 {
        return Err(anyhow!("malformed hunk header: {line}"));
    }
    let old = parse_range(parts[0].trim_start_matches('-'))?;
    let new = parse_range(parts[1].trim_start_matches('+'))?;
    Ok((old.0, old.1, new.0, new.1))
}

fn parse_range(s: &str) -> Result<(u32, u32)> {
    let mut split = s.split(',');
    let start: u32 = split
        .next()
        .ok_or_else(|| anyhow!("range start"))?
        .parse()
        .context("range start")?;
    let count: u32 = match split.next() {
        Some(c) => c.parse().context("range count")?,
        None => 1,
    };
    Ok((start, count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_unified_diff() {
        let input = "diff --git a/foo.rs b/foo.rs\nindex abc..def 100644\n--- a/foo.rs\n+++ b/foo.rs\n@@ -1,3 +1,3 @@\n line1\n-removed\n+added\n line3\n";
        let diff = parse(input).unwrap();
        assert_eq!(diff.files.len(), 1);
        let f = &diff.files[0];
        assert_eq!(f.path, "foo.rs");
        assert_eq!(f.additions, 1);
        assert_eq!(f.deletions, 1);
        assert_eq!(f.hunks.len(), 1);
        assert_eq!(f.hunks[0].lines.len(), 4);
    }

    #[test]
    fn parses_multiple_files() {
        let input = "diff --git a/a.txt b/a.txt\nindex 1..2 100644\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/b.txt b/b.txt\nindex 3..4 100644\n--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-x\n+y\n";
        let diff = parse(input).unwrap();
        assert_eq!(diff.files.len(), 2);
    }
}
