use crate::diff;
use crate::diff::types::Diff;
use anyhow::{Context, Result, anyhow};
use git2::Repository;
use std::path::PathBuf;

pub struct BranchSession {
    pub base_ref: String,
    pub head_ref: String,
    pub merge_base_short: String,
    pub diff: Diff,
}

pub fn open(repo_path: PathBuf, branch: Option<String>) -> Result<BranchSession> {
    let repo = Repository::open(&repo_path).context("open repo")?;

    let head_ref = match branch {
        Some(b) => b,
        None => current_branch(&repo)?,
    };

    let base_ref = pick_base_ref(&repo)?;

    let base_oid = repo.revparse_single(&base_ref)?.peel_to_commit()?.id();
    let head_oid = repo.revparse_single(&head_ref)?.peel_to_commit()?.id();
    let merge_base_oid = repo.merge_base(base_oid, head_oid)?;
    let merge_base_short = format!("{merge_base_oid}").chars().take(7).collect();

    let diff = diff::compute::compute(&repo_path, &base_ref, &head_ref)?;

    Ok(BranchSession {
        base_ref,
        head_ref,
        merge_base_short,
        diff,
    })
}

fn current_branch(repo: &Repository) -> Result<String> {
    let head = repo.head().context("HEAD")?;
    head.shorthand()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("HEAD has no shorthand (detached?)"))
}

fn pick_base_ref(repo: &Repository) -> Result<String> {
    for candidate in [
        "refs/remotes/origin/main",
        "refs/remotes/origin/master",
        "refs/heads/main",
        "refs/heads/master",
    ] {
        if repo.find_reference(candidate).is_ok() {
            return Ok(candidate.to_string());
        }
    }
    Err(anyhow!("no main/master ref found (looked under refs/remotes/origin/ and refs/heads/)"))
}
