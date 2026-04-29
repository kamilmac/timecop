use crate::diff::types::Diff;
use crate::session::Overlay;
use anyhow::Result;

pub struct PrSession {
    pub number: u32,
    pub head_ref: String,
    pub base_ref: String,
    pub diff: Diff,
    pub overlay: Overlay,
}

pub fn open(_pr_number: u32) -> Result<PrSession> {
    todo!()
}
