pub mod branch;
pub mod pr;

use crate::diff::types::Diff;

pub enum Session {
    Branch(branch::BranchSession),
    Pr(pr::PrSession),
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
    pub author: String,
    pub created_at: String,
    pub body: String,
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
