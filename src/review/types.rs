pub type ThreadId = String;

pub struct Draft {
    pub new_comments: Vec<NewComment>,
    pub replies: Vec<Reply>,
    pub resolutions: Vec<ThreadId>,
    pub verdict: Option<Verdict>,
}

pub struct NewComment {
    pub file: String,
    pub line: u32,
    pub body: String,
}

pub struct Reply {
    pub thread_id: ThreadId,
    pub body: String,
}

pub enum Verdict {
    Approve,
    RequestChanges,
    Comment,
}
