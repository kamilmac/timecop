use crate::diff::types::Diff;
use anyhow::Result;
use std::path::PathBuf;

pub struct BranchSession {
    pub repo: PathBuf,
    pub base_ref: String,
    pub head_ref: String,
    pub diff: Diff,
}

pub fn open(_repo: PathBuf, _branch: Option<String>) -> Result<BranchSession> {
    todo!()
}
