pub mod branch;
pub mod pr;

pub enum Session {
    Branch(branch::BranchSession),
    Pr(pr::PrSession),
}

pub struct Overlay {
    pub description: String,
    pub threads: Vec<Thread>,
}

pub struct Thread {
    pub id: String,
    pub file: String,
    pub line: u32,
    pub comments: Vec<ExistingComment>,
    pub resolved: bool,
    pub outdated: bool,
}

pub struct ExistingComment {
    pub author: String,
    pub created_at: String,
    pub body: String,
}
