//! Widget actions - what widgets report happened
//!
//! These actions define the interface between widgets and App.

use std::path::PathBuf;

/// Actions that widgets can return from key handling.
/// App dispatches these to update other state.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// No action, key was handled internally
    None,

    /// Key was not handled, pass to parent
    Ignored,

    // Navigation
    /// Request focus change
    ChangeFocus(FocusTarget),

    // File list actions
    /// File was selected (Enter on file)
    FileSelected(PathBuf),

    // PR list actions
    /// PR was selected
    PrSelected(u64),
    /// Checkout PR (Enter on PR)
    CheckoutPr(u64),
    /// Open PR in browser
    OpenPrInBrowser(u64),

    // Review actions
    /// Open review modal
    OpenReviewModal(ReviewActionType),

    // Global
    /// Quit the application
    Quit,
    /// Refresh data
    Refresh,
    /// Toggle help modal
    ToggleHelp,
    /// Yank path to clipboard
    YankPath,
    /// Open file in editor
    OpenInEditor,
    /// Navigate timeline
    TimelineNext,
    TimelinePrev,
}

/// Focus targets for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    FileList,
    PrList,
    Preview,
    Next,
    Prev,
}

/// Types of review actions
#[derive(Debug, Clone, PartialEq)]
pub enum ReviewActionType {
    Approve { pr_number: u64 },
    RequestChanges { pr_number: u64 },
    Comment { pr_number: u64 },
    LineComment { pr_number: u64, path: String, line: u32 },
}
