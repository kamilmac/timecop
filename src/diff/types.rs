#[derive(Debug, Clone)]
pub struct Diff {
    pub files: Vec<File>,
}

#[derive(Debug, Clone)]
pub struct File {
    pub path: String,
    pub hunks: Vec<Hunk>,
    pub additions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub kind: LineKind,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Context,
    Added,
    Removed,
}

impl Diff {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn file(&self, path: &str) -> Option<&File> {
        self.files.iter().find(|f| f.path == path)
    }
}

impl Default for Diff {
    fn default() -> Self {
        Self::new()
    }
}
