use crate::diff::types::{Diff, File, Hunk, Line, LineKind};
use anyhow::{Context, Result, anyhow};
use git2::{DiffOptions, Repository};
use std::path::Path;

pub fn compute(repo_path: &Path, base_ref: &str, head_ref: &str) -> Result<Diff> {
    let repo = Repository::open(repo_path).context("open repo")?;

    let base_oid = repo
        .revparse_single(base_ref)
        .with_context(|| format!("resolve base ref {base_ref}"))?
        .peel_to_commit()?
        .id();
    let head_oid = repo
        .revparse_single(head_ref)
        .with_context(|| format!("resolve head ref {head_ref}"))?
        .peel_to_commit()?
        .id();

    let merge_base = repo
        .merge_base(base_oid, head_oid)
        .context("merge-base")?;

    let base_tree = repo.find_commit(merge_base)?.tree()?;
    let head_tree = repo.find_commit(head_oid)?.tree()?;

    let mut opts = DiffOptions::new();
    opts.context_lines(3);
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut opts))?;

    let mut files: Vec<File> = Vec::new();
    let num_deltas = diff.deltas().count();

    for idx in 0..num_deltas {
        let patch = match git2::Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        let delta = patch.delta();
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .ok_or_else(|| anyhow!("delta with no path"))?
            .to_string_lossy()
            .into_owned();

        let mut additions = 0u32;
        let mut deletions = 0u32;
        let mut hunks: Vec<Hunk> = Vec::new();

        let num_hunks = patch.num_hunks();
        for h in 0..num_hunks {
            let (hunk_info, num_lines) = patch.hunk(h)?;
            let header = String::from_utf8_lossy(hunk_info.header()).trim_end().to_string();
            let mut lines: Vec<Line> = Vec::with_capacity(num_lines);
            for l in 0..num_lines {
                let line = patch.line_in_hunk(h, l)?;
                let kind = match line.origin() {
                    '+' => LineKind::Added,
                    '-' => LineKind::Removed,
                    _ => LineKind::Context,
                };
                let text = String::from_utf8_lossy(line.content())
                    .trim_end_matches('\n')
                    .to_string();
                match kind {
                    LineKind::Added => additions += 1,
                    LineKind::Removed => deletions += 1,
                    LineKind::Context => {}
                }
                lines.push(Line {
                    kind,
                    old_lineno: line.old_lineno(),
                    new_lineno: line.new_lineno(),
                    text,
                });
            }
            hunks.push(Hunk {
                header,
                new_start: hunk_info.new_start(),
                new_count: hunk_info.new_lines(),
                lines,
            });
        }

        files.push(File { path, hunks, additions, deletions });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Diff { files })
}
