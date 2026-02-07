use std::fmt;

/// File status in git
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Unchanged,
}

impl FileStatus {
    pub fn as_char(&self) -> char {
        match self {
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
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


/// Diff statistics
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
}

/// Timeline position for viewing PR history
/// Order (older → newer): -16 → ... → -1 → Wip → FullDiff → Browse
/// FullDiff is the default (primary code review view)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelinePosition {
    /// View changes from a single commit: HEAD~N → HEAD~(N-1)
    CommitDiff(usize),
    /// View only uncommitted changes: HEAD → working tree
    Wip,
    /// View all committed changes: base → HEAD (default)
    #[default]
    FullDiff,
    /// Browse all files in repo (not just changed files)
    Browse,
}

impl TimelinePosition {
    /// Move to next position (towards newer: -16 → ... → -1 → Wip → FullDiff → Browse)
    pub fn next(self, _max_commits: usize) -> Self {
        match self {
            Self::CommitDiff(1) => Self::Wip,
            Self::CommitDiff(n) => Self::CommitDiff(n - 1),
            Self::Wip => Self::FullDiff,
            Self::FullDiff => Self::Browse,
            Self::Browse => Self::Browse, // Can't go newer than browse
        }
    }

    /// Move to previous position (towards older: Browse → FullDiff → Wip → -1 → ... → -16)
    pub fn prev(self, max_commits: usize) -> Self {
        match self {
            Self::Browse => Self::FullDiff,
            Self::FullDiff => Self::Wip,
            Self::Wip => {
                if max_commits > 0 {
                    Self::CommitDiff(1)
                } else {
                    Self::Wip
                }
            }
            Self::CommitDiff(n) if n < max_commits && n < 16 => Self::CommitDiff(n + 1),
            other => other,
        }
    }
}
