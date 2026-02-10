//! Widget actions - what widgets report happened
//!
//! These actions define the interface between widgets and App.

use std::path::PathBuf;

/// Type of review action being performed
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewAction {
    Approve { pr_number: u64 },
    RequestChanges { pr_number: u64 },
    Comment { pr_number: u64 },
    LineComment { pr_number: u64, path: String, line: u32 },
    ReplyToComment { pr_number: u64, comment_id: u64, author: String },
}

impl ReviewAction {
    pub fn title(&self) -> String {
        match self {
            Self::Approve { pr_number } => format!("Approve PR #{}", pr_number),
            Self::RequestChanges { pr_number } => format!("Request Changes - PR #{}", pr_number),
            Self::Comment { pr_number } => format!("Comment on PR #{}", pr_number),
            Self::LineComment { pr_number, path, line } => {
                format!("Comment on {}:{} - PR #{}", path, line, pr_number)
            }
            Self::ReplyToComment { pr_number, author, .. } => {
                format!("Reply to {}'s comment — PR #{}", author, pr_number)
            }
        }
    }

    pub fn needs_body(&self) -> bool {
        matches!(self, Self::RequestChanges { .. } | Self::Comment { .. } | Self::LineComment { .. } | Self::ReplyToComment { .. })
    }

    pub fn confirmation_message(&self) -> Option<&str> {
        match self {
            Self::Approve { .. } => Some("Are you sure you want to approve this PR?"),
            _ => None,
        }
    }
}

/// Actions that widgets can return from key handling.
/// App dispatches these to update other state.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// No action, key was handled internally
    None,

    /// Key was not handled, pass to parent
    Ignored,

    // File list actions
    /// File was selected (Enter on file)
    FileSelected(PathBuf),

    // PR list actions
    /// PR was selected
    PrSelected(u64),
    /// Checkout PR
    CheckoutPr(u64),

    /// Expand an ignored directory (lazy load its contents)
    ExpandIgnoredDir(String),

    // Review actions
    /// Open review modal
    OpenReviewModal(ReviewAction),
}
