pub type ThreadId = String;

#[derive(Debug, Clone, Default)]
pub struct Draft {
    pub new_comments: Vec<NewComment>,
    pub replies: Vec<Reply>,
    pub resolutions: Vec<ThreadId>,
    pub verdict: Option<Verdict>,
}

#[derive(Debug, Clone)]
pub struct NewComment {
    pub file: String,
    pub line: u32,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct Reply {
    pub thread_id: ThreadId,
    pub body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Approve,
    RequestChanges,
    Comment,
}

impl Draft {
    pub fn is_empty(&self) -> bool {
        self.new_comments.is_empty()
            && self.replies.is_empty()
            && self.resolutions.is_empty()
    }
}
