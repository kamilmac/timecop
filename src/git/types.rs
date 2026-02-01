use std::fmt;

/// File status in git
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Unchanged,
}

impl FileStatus {
    pub fn as_char(&self) -> char {
        match self {
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Untracked => '?',
            Self::Unchanged => ' ',
        }
    }
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

/// A file with its status
#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub status: FileStatus,
}

/// Diff mode - what to compare against
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffMode {
    /// Uncommitted changes only (git diff)
    Working,
    /// All changes vs base branch (git diff <base>)
    #[default]
    Branch,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    /// Show uncommitted changes (mode 1)
    ChangedWorking,
    /// Show all changes vs base branch (mode 2) - default
    #[default]
    ChangedBranch,
    /// Browse all files (mode 3)
    Browse,
    /// Browse docs/markdown only (mode 4)
    Docs,
}

impl AppMode {
    pub fn next(self) -> Self {
        match self {
            Self::ChangedWorking => Self::ChangedBranch,
            Self::ChangedBranch => Self::Browse,
            Self::Browse => Self::Docs,
            Self::Docs => Self::ChangedWorking,
        }
    }

    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::ChangedWorking),
            2 => Some(Self::ChangedBranch),
            3 => Some(Self::Browse),
            4 => Some(Self::Docs),
            _ => None,
        }
    }

    pub fn diff_mode(&self) -> DiffMode {
        match self {
            Self::ChangedWorking => DiffMode::Working,
            _ => DiffMode::Branch,
        }
    }

    pub fn is_changed_mode(&self) -> bool {
        matches!(self, Self::ChangedWorking | Self::ChangedBranch)
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::ChangedWorking => "working",
            Self::ChangedBranch => "branch",
            Self::Browse => "browse",
            Self::Docs => "docs",
        }
    }
}

impl fmt::Display for AppMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

/// A git commit
#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

/// Diff statistics
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
}
