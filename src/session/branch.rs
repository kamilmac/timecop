use crate::diff;
use crate::diff::types::Diff;
use anyhow::{Context, Result, anyhow};
use git2::Repository;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchView {
    VsBase,
    Uncommitted,
}

pub struct BranchSession {
    pub base_ref: String,
    pub head_ref: String,
    pub merge_base_short: String,
    pub diff: Diff,
    pub is_current: bool,
    pub view: BranchView,
}

pub fn open(repo_path: PathBuf, branch: Option<String>) -> Result<BranchSession> {
    open_with_view(repo_path, branch, BranchView::VsBase)
}

pub fn open_with_view(
    repo_path: PathBuf,
    branch: Option<String>,
    view: BranchView,
) -> Result<BranchSession> {
    let repo = Repository::open(&repo_path).context("open repo")?;

    let current = current_branch(&repo)?;
    let (head_ref, is_current) = match branch {
        None => (current.clone(), true),
        Some(b) if b == current => (current.clone(), true),
        Some(b) => (b, false),
    };

    let base_ref = pick_base_ref(&repo)?;

    let base_oid = repo.revparse_single(&base_ref)?.peel_to_commit()?.id();
    let head_oid = repo.revparse_single(&head_ref)?.peel_to_commit()?.id();
    let merge_base_oid = repo.merge_base(base_oid, head_oid)?;
    let merge_base_short = format!("{merge_base_oid}").chars().take(7).collect();

    let diff = match view {
        BranchView::VsBase if is_current => diff::compute::compute_workdir(&repo_path, &base_ref)?,
        BranchView::VsBase => diff::compute::compute(&repo_path, &base_ref, &head_ref)?,
        BranchView::Uncommitted => diff::compute::compute_workdir(&repo_path, "HEAD")?,
    };

    Ok(BranchSession {
        base_ref,
        head_ref,
        merge_base_short,
        diff,
        is_current,
        view,
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
