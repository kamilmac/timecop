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
/// Order: FullDiff → Wip → -1 → -2 → ... → -8
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimelinePosition {
    /// View all committed changes: base → HEAD (default)
    #[default]
    FullDiff,
    /// View only uncommitted changes: HEAD → working tree
    Wip,
    /// View changes from a single commit: HEAD~N → HEAD~(N-1)
    CommitDiff(usize),
}

impl TimelinePosition {
    /// Move to next position (towards older commits)
    pub fn next(self, max_commits: usize) -> Self {
        match self {
            Self::FullDiff => Self::Wip,
            Self::Wip => {
                if max_commits > 0 {
                    Self::CommitDiff(1)
                } else {
                    Self::Wip
                }
            }
            Self::CommitDiff(n) if n < max_commits && n < 7 => Self::CommitDiff(n + 1),
            other => other,
        }
    }

    /// Move to previous position (towards full diff)
    pub fn prev(self) -> Self {
        match self {
            Self::FullDiff => Self::FullDiff,
            Self::Wip => Self::FullDiff,
            Self::CommitDiff(1) => Self::Wip,
            Self::CommitDiff(n) => Self::CommitDiff(n - 1),
        }
    }

    /// Get index for timeline display (0 = full diff)
    pub fn display_index(&self) -> usize {
        match self {
            Self::FullDiff => 0,
            Self::Wip => 1,
            Self::CommitDiff(n) => n + 1,
        }
    }
}
