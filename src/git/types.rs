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

/// Type of entry in file listing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntryType {
    /// Normal tracked file
    #[default]
    Tracked,
    /// Gitignored file (Browse mode only)
    Ignored,
    /// Gitignored directory - shown but not recursed into (Browse mode only)
    IgnoredDir,
}

impl EntryType {
    pub fn is_ignored(self) -> bool {
        matches!(self, Self::Ignored | Self::IgnoredDir)
    }

    pub fn is_dir(self) -> bool {
        matches!(self, Self::IgnoredDir)
    }
}

/// A file or directory entry with its status
#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub status: FileStatus,
    /// True if file has uncommitted changes (diff modes only)
    pub uncommitted: bool,
    /// Entry type - tracked, ignored file, or ignored directory
    pub entry_type: EntryType,
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
    pub fn next(self) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- FileStatus ---

    #[test]
    fn file_status_as_char() {
        assert_eq!(FileStatus::Modified.as_char(), 'M');
        assert_eq!(FileStatus::Added.as_char(), 'A');
        assert_eq!(FileStatus::Deleted.as_char(), 'D');
        assert_eq!(FileStatus::Renamed.as_char(), 'R');
        assert_eq!(FileStatus::Unchanged.as_char(), ' ');
    }

    #[test]
    fn file_status_display() {
        assert_eq!(format!("{}", FileStatus::Modified), "M");
        assert_eq!(format!("{}", FileStatus::Unchanged), " ");
    }

    // --- EntryType ---

    #[test]
    fn entry_type_is_ignored() {
        assert!(!EntryType::Tracked.is_ignored());
        assert!(EntryType::Ignored.is_ignored());
        assert!(EntryType::IgnoredDir.is_ignored());
    }

    #[test]
    fn entry_type_is_dir() {
        assert!(!EntryType::Tracked.is_dir());
        assert!(!EntryType::Ignored.is_dir());
        assert!(EntryType::IgnoredDir.is_dir());
    }

    // --- TimelinePosition::next ---

    #[test]
    fn timeline_next_full_traversal() {
        let pos = TimelinePosition::CommitDiff(3);
        let pos = pos.next(); // CommitDiff(2)
        assert_eq!(pos, TimelinePosition::CommitDiff(2));
        let pos = pos.next(); // CommitDiff(1)
        assert_eq!(pos, TimelinePosition::CommitDiff(1));
        let pos = pos.next(); // Wip
        assert_eq!(pos, TimelinePosition::Wip);
        let pos = pos.next(); // FullDiff
        assert_eq!(pos, TimelinePosition::FullDiff);
        let pos = pos.next(); // Browse
        assert_eq!(pos, TimelinePosition::Browse);
        let pos = pos.next(); // stays Browse
        assert_eq!(pos, TimelinePosition::Browse);
    }

    // --- TimelinePosition::prev ---

    #[test]
    fn timeline_prev_full_traversal() {
        let pos = TimelinePosition::Browse;
        let pos = pos.prev(3);
        assert_eq!(pos, TimelinePosition::FullDiff);
        let pos = pos.prev(3);
        assert_eq!(pos, TimelinePosition::Wip);
        let pos = pos.prev(3);
        assert_eq!(pos, TimelinePosition::CommitDiff(1));
        let pos = pos.prev(3);
        assert_eq!(pos, TimelinePosition::CommitDiff(2));
        let pos = pos.prev(3);
        assert_eq!(pos, TimelinePosition::CommitDiff(3));
        let pos = pos.prev(3); // capped at max_commits
        assert_eq!(pos, TimelinePosition::CommitDiff(3));
    }

    #[test]
    fn timeline_prev_no_commits() {
        let pos = TimelinePosition::Wip.prev(0);
        assert_eq!(pos, TimelinePosition::Wip); // can't go further
    }

    #[test]
    fn timeline_prev_capped_at_16() {
        let pos = TimelinePosition::CommitDiff(16).prev(100);
        assert_eq!(pos, TimelinePosition::CommitDiff(16)); // 16 is hard max
    }

    #[test]
    fn timeline_default_is_full_diff() {
        assert_eq!(TimelinePosition::default(), TimelinePosition::FullDiff);
    }
}
