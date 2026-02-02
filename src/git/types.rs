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
    /// True if file has uncommitted changes
    pub uncommitted: bool,
}

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    /// Show all changes vs base branch + uncommitted (mode 1) - default
    #[default]
    Changes,
    /// Browse all files (mode 2)
    Browse,
    /// Browse docs/markdown only (mode 3)
    Docs,
}

impl AppMode {
    pub fn next(self) -> Self {
        match self {
            Self::Changes => Self::Browse,
            Self::Browse => Self::Docs,
            Self::Docs => Self::Changes,
        }
    }

    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Changes),
            2 => Some(Self::Browse),
            3 => Some(Self::Docs),
            _ => None,
        }
    }

    pub fn is_changed_mode(&self) -> bool {
        matches!(self, Self::Changes)
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Changes => "changes",
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

/// Diff statistics
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
}

/// Timeline position for viewing PR history
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelinePosition {
    /// View diff at N commits before HEAD (0 = HEAD)
    Commit(usize),
    /// View only uncommitted changes (working tree vs HEAD)
    #[default]
    Uncommitted,
}

impl TimelinePosition {
    /// Move back in time (towards older commits)
    pub fn back(self, max_commits: usize) -> Self {
        match self {
            Self::Uncommitted => Self::Commit(0),
            Self::Commit(n) if n < max_commits => Self::Commit(n + 1),
            other => other,
        }
    }

    /// Move forward in time (towards uncommitted)
    pub fn forward(self) -> Self {
        match self {
            Self::Commit(0) => Self::Uncommitted,
            Self::Commit(n) => Self::Commit(n - 1),
            Self::Uncommitted => Self::Uncommitted,
        }
    }

    /// Get display label
    pub fn label(&self) -> String {
        match self {
            Self::Uncommitted => "wip".to_string(),
            Self::Commit(0) => "HEAD".to_string(),
            Self::Commit(n) => format!("-{}", n),
        }
    }
}
