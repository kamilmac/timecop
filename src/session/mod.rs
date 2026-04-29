pub mod branch;
pub mod pr;

use crate::diff::types::Diff;

pub enum Session {
    Branch(branch::BranchSession),
    Pr(pr::PrSession),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approve,
    RequestChanges,
}

#[derive(Debug, Clone)]
pub struct Overlay {
    pub description: String,
    pub threads: Vec<Thread>,
}

#[derive(Debug, Clone)]
pub struct Thread {
    pub node_id: String,
    pub root_comment_id: u64,
    pub file: String,
    pub line: u32,
    pub comments: Vec<ExistingComment>,
    pub resolved: bool,
    pub outdated: bool,
}

#[derive(Debug, Clone)]
pub struct ExistingComment {
    pub id: u64,
    pub author: String,
    pub created_at: String,
    pub body: String,
    pub reactions: Reactions,
}

#[derive(Debug, Clone, Default)]
pub struct Reactions {
    pub thumbs_up: u32,
    pub thumbs_down: u32,
    pub heart: u32,
    pub hooray: u32,
    pub laugh: u32,
    pub confused: u32,
    pub rocket: u32,
    pub eyes: u32,
}

impl Reactions {
    pub fn is_empty(&self) -> bool {
        self.thumbs_up == 0
            && self.thumbs_down == 0
            && self.heart == 0
            && self.hooray == 0
            && self.laugh == 0
            && self.confused == 0
            && self.rocket == 0
            && self.eyes == 0
    }
}

impl Session {
    pub fn diff(&self) -> &Diff {
        match self {
            Session::Branch(b) => &b.diff,
            Session::Pr(p) => &p.diff,
        }
    }

    pub fn overlay(&self) -> Option<&Overlay> {
        match self {
            Session::Branch(_) => None,
            Session::Pr(p) => Some(&p.overlay),
        }
    }
}
