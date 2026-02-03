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
    /// True if this is a suggested related file (co-change analysis)
    pub suggested: bool,
}


/// Diff statistics
#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub added: usize,
    pub removed: usize,
}

/// Timeline position for viewing PR history
/// Visual order (left to right): -N → ... → -1 → [all] → [all+] → [wip]
/// Navigation: , moves left (older), . moves right (newer)
/// FullDiff is the default (primary code review view)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelinePosition {
    /// View only uncommitted changes: HEAD → working tree
    Wip,
    /// View all changes + suggested related files (co-change analysis)
    FullDiffExtended,
    /// View all committed changes: base → HEAD (default)
    #[default]
    FullDiff,
    /// View changes from a single commit: HEAD~N → HEAD~(N-1)
    CommitDiff(usize),
}

impl TimelinePosition {
    /// Move to next position (towards older/left: Wip → FullDiffExtended → FullDiff → -1 → ...)
    pub fn next(self, max_commits: usize) -> Self {
        match self {
            Self::Wip => Self::FullDiffExtended,
            Self::FullDiffExtended => Self::FullDiff,
            Self::FullDiff => {
                if max_commits > 0 {
                    Self::CommitDiff(1)
                } else {
                    Self::FullDiff
                }
            }
            Self::CommitDiff(n) if n < max_commits && n < 16 => Self::CommitDiff(n + 1),
            other => other,
        }
    }

    /// Move to previous position (towards newer/right: ... → -1 → FullDiff → FullDiffExtended → Wip)
    pub fn prev(self) -> Self {
        match self {
            Self::Wip => Self::Wip, // Can't go newer than wip
            Self::FullDiffExtended => Self::Wip,
            Self::FullDiff => Self::FullDiffExtended,
            Self::CommitDiff(1) => Self::FullDiff,
            Self::CommitDiff(n) => Self::CommitDiff(n - 1),
        }
    }
}
