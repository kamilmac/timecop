pub struct Diff {
    pub files: Vec<File>,
}

pub struct File {
    pub path: String,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
}

pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub new_start: u32,
    pub lines: Vec<Line>,
}

pub struct Line {
    pub kind: LineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub text: String,
}

pub enum LineKind {
    Context,
    Added,
    Removed,
}
