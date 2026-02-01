use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    Frame,
};
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::event::KeyInput;
use crate::git::{AppMode, Commit, DiffStats, GitClient, StatusEntry};
use crate::github::{GitHubClient, PrInfo, PrSummary};
use crate::ui::{
    centered_rect, AppLayout, DiffView, DiffViewState, FileList, FileListState, HelpModal,
    Highlighter, InputModal, InputModalState, InputResult, PrListPanel, PrListPanelState,
    PreviewContent, ReviewAction,
};

/// Which window is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedWindow {
    FileList,
    PrList,
    Preview,
}

impl FocusedWindow {
    /// Tab cycles between left panes only
    pub fn next_left(self) -> Self {
        match self {
            Self::FileList => Self::PrList,
            Self::PrList => Self::FileList,
            Self::Preview => Self::FileList,
        }
    }

    pub fn prev_left(self) -> Self {
        match self {
            Self::FileList => Self::PrList,
            Self::PrList => Self::FileList,
            Self::Preview => Self::PrList,
        }
    }

    pub fn is_left_pane(self) -> bool {
        matches!(self, Self::FileList | Self::PrList)
    }
}

/// Command to execute after handling input
#[derive(Debug, Clone)]
pub enum AppCommand {
    None,
    OpenEditor { path: String, line: Option<usize> },
}

/// Main application state
pub struct App {
    // Core
    pub running: bool,
    pub config: Config,
    git: GitClient,
    github: GitHubClient,
    repo_path: String,

    // State
    pub mode: AppMode,
    pub focused: FocusedWindow,
    pub show_help: bool,
    pub pending_command: AppCommand,

    // Data
    pub branch: String,
    pub base_branch: Option<String>,
    pub files: Vec<StatusEntry>,
    pub commits: Vec<Commit>,
    pub diff_stats: DiffStats,
    stats_loading: bool,
    stats_rx: Option<Receiver<DiffStats>>,
    // Full PR details for currently selected PR
    pub selected_pr: Option<PrInfo>,
    selected_pr_number: Option<u64>,
    pr_detail_loading: bool,
    pr_detail_rx: Option<Receiver<Option<PrInfo>>>,
    // PR list polling
    last_pr_list_poll: Instant,
    pr_list_loading: bool,
    pr_list_rx: Option<Receiver<Vec<PrSummary>>>,

    // Widget states
    pub file_list_state: FileListState,
    pub pr_list_panel_state: PrListPanelState,
    pub diff_view_state: DiffViewState,
    pub input_modal_state: InputModalState,

    // Syntax highlighting
    highlighter: Highlighter,
}

impl App {
    pub fn new(path: &str) -> Result<Self> {
        let git = GitClient::open(path)?;
        let github = GitHubClient::new();

        let branch = git.current_branch().unwrap_or_else(|_| "HEAD".to_string());
        let base_branch = git.base_branch().map(String::from);

        let mut app = Self {
            running: true,
            config: Config::default(),
            git,
            github,
            repo_path: path.to_string(),
            mode: AppMode::default(),
            focused: FocusedWindow::FileList,
            show_help: false,
            pending_command: AppCommand::None,
            branch,
            base_branch,
            files: vec![],
            commits: vec![],
            diff_stats: DiffStats::default(),
            stats_loading: false,
            stats_rx: None,
            selected_pr: None,
            selected_pr_number: None,
            pr_detail_loading: false,
            pr_detail_rx: None,
            last_pr_list_poll: Instant::now() - Duration::from_secs(301), // Force immediate load
            pr_list_loading: false,
            pr_list_rx: None,
            file_list_state: FileListState::new(),
            pr_list_panel_state: PrListPanelState::new(),
            diff_view_state: DiffViewState::new(),
            input_modal_state: InputModalState::new(),
            highlighter: Highlighter::new(),
        };

        // Initialize PR list panel with current branch
        app.pr_list_panel_state.set_current_branch(app.branch.clone());

        app.refresh()?;
        Ok(app)
    }

    /// Refresh all data from git
    pub fn refresh(&mut self) -> Result<()> {
        // Update branch name
        let new_branch = self.git.current_branch().unwrap_or_else(|_| "HEAD".to_string());
        let branch_changed = new_branch != self.branch;
        self.branch = new_branch;

        // Load files based on mode
        self.files = match self.mode {
            AppMode::Browse => self.git.list_all_files()?,
            AppMode::Docs => self.git.list_doc_files()?,
            _ => self.git.status(self.mode.diff_mode())?,
        };

        // Load commits
        self.commits = self.git.log(self.config.layout.max_commits)?;

        // Trigger async stats loading
        self.diff_stats = DiffStats::default();
        if self.mode.is_changed_mode() && !self.stats_loading {
            self.spawn_stats_loader();
        }

        // Update widget states
        self.file_list_state.set_files(self.files.clone());

        // Update PR list panel with current branch
        if branch_changed {
            self.pr_list_panel_state.set_current_branch(self.branch.clone());
            // Clear selected PR details since branch changed
            self.selected_pr = None;
            self.selected_pr_number = None;
        }

        // Update preview
        self.update_preview();

        Ok(())
    }

    /// Spawn background thread to load diff stats
    fn spawn_stats_loader(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.stats_rx = Some(rx);
        self.stats_loading = true;

        let path = self.repo_path.clone();
        let mode = self.mode.diff_mode();
        thread::spawn(move || {
            if let Ok(git) = GitClient::open(&path) {
                if let Ok(stats) = git.diff_stats(mode) {
                    let _ = tx.send(stats);
                }
            }
        });
    }

    /// Spawn background thread to load PR list
    fn spawn_pr_list_loader(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.pr_list_rx = Some(rx);
        self.pr_list_loading = true;
        self.pr_list_panel_state.loading = true;

        thread::spawn(move || {
            let mut github = GitHubClient::new();
            let prs = if github.is_available() {
                github.list_open_prs().unwrap_or_default()
            } else {
                vec![]
            };
            let _ = tx.send(prs);
        });
    }

    /// Spawn background thread to load full PR details
    fn spawn_pr_detail_loader(&mut self, pr_number: u64) {
        let (tx, rx) = mpsc::channel();
        self.pr_detail_rx = Some(rx);
        self.pr_detail_loading = true;
        self.selected_pr_number = Some(pr_number);

        thread::spawn(move || {
            let mut github = GitHubClient::new();
            if github.is_available() {
                // Use get_pr_for_branch but we need the PR number
                // For now, we'll fetch by branch - but ideally we'd have a get_pr_by_number method
                if let Ok(pr) = github.get_pr_by_number(pr_number) {
                    let _ = tx.send(pr);
                } else {
                    let _ = tx.send(None);
                }
            } else {
                let _ = tx.send(None);
            }
        });
    }

    /// Apply full PR details to state
    fn apply_pr_details(&mut self, pr: PrInfo) {
        // Update file comments indicator
        let comments: HashMap<String, bool> = pr
            .file_comments
            .keys()
            .map(|k| (k.clone(), true))
            .collect();
        self.file_list_state.set_comments(comments);

        // Update diff view with PR for inline comments
        self.diff_view_state.set_pr(Some(pr.clone()));

        self.selected_pr = Some(pr);

        // Update preview if PR list is focused
        if self.focused == FocusedWindow::PrList {
            self.show_selected_pr_in_preview();
        }
    }

    /// Handle tick event - periodic updates
    pub fn handle_tick(&mut self) {
        const PR_LIST_POLL_INTERVAL: Duration = Duration::from_secs(300); // 5 minutes

        // Check for completed stats loading
        if let Some(ref rx) = self.stats_rx {
            match rx.try_recv() {
                Ok(stats) => {
                    self.diff_stats = stats;
                    self.stats_loading = false;
                    self.stats_rx = None;
                }
                Err(TryRecvError::Disconnected) => {
                    self.stats_loading = false;
                    self.stats_rx = None;
                }
                Err(TryRecvError::Empty) => {} // Still loading
            }
        }

        // Check for completed PR list loading
        if let Some(ref rx) = self.pr_list_rx {
            match rx.try_recv() {
                Ok(prs) => {
                    self.pr_list_panel_state.set_prs(prs);
                    self.pr_list_loading = false;
                    self.pr_list_rx = None;
                    self.last_pr_list_poll = Instant::now();

                    // Auto-load details for selected PR
                    if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                        let already_loaded = self.selected_pr.as_ref().map(|p| p.number) == Some(pr_num);
                        if !already_loaded && !self.pr_detail_loading {
                            self.spawn_pr_detail_loader(pr_num);
                        }
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.pr_list_loading = false;
                    self.pr_list_rx = None;
                    self.pr_list_panel_state.loading = false;
                }
                Err(TryRecvError::Empty) => {} // Still loading
            }
        }

        // Check for completed PR detail loading
        if let Some(ref rx) = self.pr_detail_rx {
            match rx.try_recv() {
                Ok(Some(pr)) => {
                    // Only apply if this PR is still the one we want
                    let currently_selected = self.pr_list_panel_state.selected_number();
                    if currently_selected == Some(pr.number) {
                        self.apply_pr_details(pr);
                    }
                    self.pr_detail_loading = false;
                    self.pr_detail_rx = None;
                    // Update preview to show loaded content
                    if self.focused == FocusedWindow::PrList {
                        self.show_selected_pr_in_preview();
                    }
                }
                Ok(None) => {
                    self.pr_detail_loading = false;
                    self.pr_detail_rx = None;
                    // Update preview to clear loading state
                    if self.focused == FocusedWindow::PrList {
                        self.show_selected_pr_in_preview();
                    }
                }
                Err(TryRecvError::Disconnected) => {
                    self.pr_detail_loading = false;
                    self.pr_detail_rx = None;
                    // Update preview to clear loading state
                    if self.focused == FocusedWindow::PrList {
                        self.show_selected_pr_in_preview();
                    }
                }
                Err(TryRecvError::Empty) => {} // Still loading
            }
        }

        // Trigger PR list loading if needed (on startup and every 5 minutes)
        let should_load_pr_list = self.last_pr_list_poll.elapsed() >= PR_LIST_POLL_INTERVAL;
        if should_load_pr_list && !self.pr_list_loading {
            self.spawn_pr_list_loader();
        }
    }

    /// Apply default settings when switching modes
    fn apply_mode_defaults(&mut self, old_mode: AppMode) {
        let entering_browse = (self.mode == AppMode::Browse || self.mode == AppMode::Docs)
            && old_mode != AppMode::Browse
            && old_mode != AppMode::Docs;

        let leaving_browse = (old_mode == AppMode::Browse || old_mode == AppMode::Docs)
            && self.mode != AppMode::Browse
            && self.mode != AppMode::Docs;

        if entering_browse {
            // Collapse folders at configured depth
            let depth = self.config.layout.browse_collapse_depth;
            self.file_list_state.collapse_at_depth(depth);
        } else if leaving_browse {
            // Expand all when leaving browse mode
            self.file_list_state.expand_all();
        }
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        // Input modal takes highest priority
        if self.input_modal_state.visible {
            match self.input_modal_state.handle_key(key) {
                InputResult::Submit => {
                    self.submit_review_action()?;
                }
                InputResult::Cancelled => {}
                InputResult::Continue => {}
            }
            return Ok(());
        }

        // Help modal takes priority
        if self.show_help {
            if KeyInput::is_help(&key) || KeyInput::is_escape(&key) {
                self.show_help = false;
            }
            return Ok(());
        }

        // Global keys
        if KeyInput::is_quit(&key) {
            self.running = false;
            return Ok(());
        }

        if KeyInput::is_help(&key) {
            self.show_help = true;
            return Ok(());
        }

        if KeyInput::is_refresh(&key) {
            self.refresh()?;
            return Ok(());
        }

        // Tab cycles between left panes only
        if KeyInput::is_tab(&key) {
            self.focused = self.focused.next_left();
            self.on_focus_change();
            return Ok(());
        }

        if KeyInput::is_shift_tab(&key) {
            self.focused = self.focused.prev_left();
            self.on_focus_change();
            return Ok(());
        }

        // Enter is context-sensitive
        if KeyInput::is_enter(&key) {
            match self.focused {
                FocusedWindow::FileList => {
                    // Go to preview
                    self.focused = FocusedWindow::Preview;
                    self.on_focus_change();
                }
                FocusedWindow::PrList => {
                    // Checkout the selected PR
                    if let Some(pr) = self.pr_list_panel_state.selected() {
                        let _ = self.github.checkout_pr(pr.number);
                        self.refresh()?;
                    }
                }
                FocusedWindow::Preview => {}
            }
            return Ok(());
        }

        // Escape goes back to left pane
        if KeyInput::is_escape(&key) && self.focused == FocusedWindow::Preview {
            self.focused = FocusedWindow::FileList;
            self.on_focus_change();
            return Ok(());
        }

        if KeyInput::is_mode_cycle(&key) {
            let old_mode = self.mode;
            self.mode = self.mode.next();
            self.refresh()?;
            self.apply_mode_defaults(old_mode);
            return Ok(());
        }

        if let Some(n) = KeyInput::get_mode_number(&key) {
            if let Some(mode) = AppMode::from_number(n) {
                let old_mode = self.mode;
                self.mode = mode;
                self.refresh()?;
                self.apply_mode_defaults(old_mode);
            }
            return Ok(());
        }

        if KeyInput::is_yank(&key) {
            self.yank_path();
            return Ok(());
        }

        // 'o' key is context-specific
        if KeyInput::is_open(&key) {
            match self.focused {
                FocusedWindow::PrList => {
                    // Open selected PR in browser
                    if let Some(pr) = self.pr_list_panel_state.selected() {
                        let _ = self.github.open_pr_in_browser(pr.number);
                    }
                }
                _ => {
                    // Open file in editor
                    self.open_in_editor();
                }
            }
            return Ok(());
        }

        // Window-specific keys
        match self.focused {
            FocusedWindow::FileList => self.handle_file_list_key(&key)?,
            FocusedWindow::PrList => self.handle_pr_list_panel_key(&key)?,
            FocusedWindow::Preview => self.handle_preview_key(&key)?,
        }

        Ok(())
    }

    fn handle_file_list_key(&mut self, key: &KeyEvent) -> Result<()> {
        let changed = if KeyInput::is_down(key) {
            self.file_list_state.move_down();
            true
        } else if KeyInput::is_up(key) {
            self.file_list_state.move_up();
            true
        } else if KeyInput::is_fast_down(key) {
            self.file_list_state.move_down_n(5);
            true
        } else if KeyInput::is_fast_up(key) {
            self.file_list_state.move_up_n(5);
            true
        } else if KeyInput::is_top(key) {
            self.file_list_state.go_top();
            true
        } else if KeyInput::is_bottom(key) {
            self.file_list_state.go_bottom();
            true
        } else if KeyInput::is_left(key) {
            self.file_list_state.collapse();
            true
        } else if KeyInput::is_right(key) {
            self.file_list_state.expand();
            true
        } else {
            false
        };

        if changed {
            self.update_preview();
        }

        Ok(())
    }

    fn handle_pr_list_panel_key(&mut self, key: &KeyEvent) -> Result<()> {
        // Review actions
        if KeyInput::is_approve(key) {
            if let Some(pr) = self.pr_list_panel_state.selected() {
                self.input_modal_state.show(ReviewAction::Approve { pr_number: pr.number });
            }
            return Ok(());
        }

        if KeyInput::is_request_changes(key) {
            if let Some(pr) = self.pr_list_panel_state.selected() {
                self.input_modal_state.show(ReviewAction::RequestChanges { pr_number: pr.number });
            }
            return Ok(());
        }

        if KeyInput::is_comment(key) {
            if let Some(pr) = self.pr_list_panel_state.selected() {
                self.input_modal_state.show(ReviewAction::Comment { pr_number: pr.number });
            }
            return Ok(());
        }

        let selection_changed = if KeyInput::is_down(key) {
            self.pr_list_panel_state.move_down();
            true
        } else if KeyInput::is_up(key) {
            self.pr_list_panel_state.move_up();
            true
        } else {
            false
        };

        if selection_changed {
            // Load details for newly selected PR
            if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                // Check if we need to load this PR's details
                let already_loaded = self.selected_pr.as_ref().map(|p| p.number) == Some(pr_num);
                let already_loading = self.pr_detail_loading && self.selected_pr_number == Some(pr_num);

                if !already_loaded && !already_loading {
                    self.spawn_pr_detail_loader(pr_num);
                }
            }
            self.show_selected_pr_in_preview();
        }

        Ok(())
    }

    fn show_selected_pr_in_preview(&mut self) {
        // Show loading indicator if fetching PR details
        if self.pr_detail_loading {
            if let Some(pr) = self.pr_list_panel_state.selected() {
                self.diff_view_state.set_content(PreviewContent::Loading {
                    message: format!("Loading PR #{} details...", pr.number),
                });
            }
            return;
        }

        // Show selected PR details in preview
        if let Some(pr) = self.selected_pr.clone() {
            self.diff_view_state.set_content(PreviewContent::PrDetails { pr });
        } else if let Some(summary) = self.pr_list_panel_state.selected() {
            // Show basic info from summary if full details not loaded yet
            self.diff_view_state.set_content(PreviewContent::Loading {
                message: format!("PR #{}: {}", summary.number, summary.title),
            });
        } else {
            self.diff_view_state.set_content(PreviewContent::Empty);
        }
    }

    fn handle_preview_key(&mut self, key: &KeyEvent) -> Result<()> {
        // Line comment on current line (only if we have a selected PR and are viewing a file diff)
        if KeyInput::is_comment(key) {
            if let (Some(pr_num), Some(path), Some(line)) = (
                self.pr_list_panel_state.selected_number(),
                self.diff_view_state.get_current_file().map(|s| s.to_string()),
                self.diff_view_state.get_current_line_number(),
            ) {
                self.input_modal_state.show(ReviewAction::LineComment {
                    pr_number: pr_num,
                    path,
                    line: line as u32,
                });
            }
            return Ok(());
        }

        if KeyInput::is_down(key) {
            self.diff_view_state.move_down();
        } else if KeyInput::is_up(key) {
            self.diff_view_state.move_up();
        } else if KeyInput::is_fast_down(key) {
            self.diff_view_state.move_down_n(5);
        } else if KeyInput::is_fast_up(key) {
            self.diff_view_state.move_up_n(5);
        } else if KeyInput::is_page_down(key) {
            self.diff_view_state.page_down(20);
        } else if KeyInput::is_page_up(key) {
            self.diff_view_state.page_up(20);
        } else if KeyInput::is_top(key) {
            self.diff_view_state.go_top();
        } else if KeyInput::is_bottom(key) {
            self.diff_view_state.go_bottom();
        }

        Ok(())
    }

    fn on_focus_change(&mut self) {
        if self.focused == FocusedWindow::PrList {
            // Show selected PR in preview when PR list is focused
            self.show_selected_pr_in_preview();
            // Load details if needed
            if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                let already_loaded = self.selected_pr.as_ref().map(|p| p.number) == Some(pr_num);
                let already_loading = self.pr_detail_loading && self.selected_pr_number == Some(pr_num);

                if !already_loaded && !already_loading {
                    self.spawn_pr_detail_loader(pr_num);
                }
            }
        } else {
            self.update_preview();
        }
    }

    fn update_preview(&mut self) {
        if self.focused == FocusedWindow::PrList {
            self.show_selected_pr_in_preview();
            return;
        }

        let content = if let Some(entry) = self.file_list_state.selected() {
            if entry.is_root {
                // Root selected - show empty or PR summary
                PreviewContent::Empty
            } else if entry.is_dir && (self.mode == AppMode::Browse || self.mode == AppMode::Docs) {
                // Directory selected in browse/docs mode - show empty
                PreviewContent::Empty
            } else if entry.is_dir {
                // Directory selected in diff mode - combined diff
                let diff = self
                    .git
                    .diff_files(&entry.children, self.mode.diff_mode())
                    .unwrap_or_default();
                PreviewContent::FolderDiff {
                    path: entry.path.clone(),
                    content: diff,
                }
            } else if self.mode == AppMode::Browse || self.mode == AppMode::Docs {
                // Browse/docs mode - file content with syntax highlighting
                let file_content = self.git.read_file(&entry.path).unwrap_or_default();
                let content = PreviewContent::FileContent {
                    path: entry.path.clone(),
                    content: file_content,
                };
                self.diff_view_state.set_content_highlighted(content, &self.highlighter);
                return;
            } else {
                // Changed mode - diff with syntax highlighting
                let diff = self
                    .git
                    .diff(&entry.path, self.mode.diff_mode())
                    .unwrap_or_default();
                let content = PreviewContent::FileDiff {
                    path: entry.path.clone(),
                    content: diff,
                };
                self.diff_view_state.set_content_highlighted(content, &self.highlighter);
                return;
            }
        } else {
            PreviewContent::Empty
        };

        self.diff_view_state.set_content(content);
    }

    fn yank_path(&self) {
        let path = if self.focused == FocusedWindow::Preview {
            // Get path with line number from diff view
            let path = match &self.diff_view_state.content {
                PreviewContent::FileDiff { path, .. } => path.clone(),
                PreviewContent::FileContent { path, .. } => path.clone(),
                _ => return,
            };
            if let Some(line) = self.diff_view_state.get_current_line_number() {
                format!("{}:{}", path, line)
            } else {
                path
            }
        } else if let Some(entry) = self.file_list_state.selected() {
            entry.path.clone()
        } else {
            return;
        };

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(&path);
        }
    }

    fn open_in_editor(&mut self) {
        let (path, line) = match self.focused {
            FocusedWindow::Preview => {
                // Get line number from diff view
                let line = self.diff_view_state.get_current_line_number();
                if let Some(entry) = self.file_list_state.selected() {
                    if entry.is_dir {
                        return;
                    }
                    (entry.path.clone(), line)
                } else {
                    return;
                }
            }
            _ => {
                if let Some(entry) = self.file_list_state.selected() {
                    if entry.is_dir {
                        return;
                    }
                    (entry.path.clone(), None)
                } else {
                    return;
                }
            }
        };

        let full_path = self.git.path().join(&path);
        self.pending_command = AppCommand::OpenEditor {
            path: full_path.to_string_lossy().to_string(),
            line,
        };
    }

    /// Take pending command (clears it)
    pub fn take_command(&mut self) -> AppCommand {
        std::mem::replace(&mut self.pending_command, AppCommand::None)
    }

    /// Submit the review action from the input modal
    fn submit_review_action(&mut self) -> Result<()> {
        let Some(action) = self.input_modal_state.action.clone() else {
            return Ok(());
        };

        let body = self.input_modal_state.take_input();

        let result = match &action {
            ReviewAction::Approve { pr_number } => {
                self.github.approve_pr(*pr_number)
            }
            ReviewAction::RequestChanges { pr_number } => {
                self.github.request_changes(*pr_number, &body)
            }
            ReviewAction::Comment { pr_number } => {
                self.github.comment_pr(*pr_number, &body)
            }
            ReviewAction::LineComment { pr_number, path, line } => {
                self.github.add_line_comment(*pr_number, path, *line, &body)
            }
        };

        match result {
            Ok(()) => {
                self.input_modal_state.hide();
                // Refresh PR details to show the new comment/review
                if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                    self.spawn_pr_detail_loader(pr_num);
                }
            }
            Err(e) => {
                self.input_modal_state.set_error(format!("Error: {}", e));
            }
        }

        Ok(())
    }

    /// Render the UI
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let colors = &self.config.colors;
        let layout = AppLayout::default();
        let areas = layout.compute(area);

        // Render file list
        let file_list = FileList::new(colors)
            .focused(self.focused == FocusedWindow::FileList)
            .title(format!(
                "{} ({})",
                if self.mode == AppMode::Browse { "Browse" } else { "Files" },
                self.files.len()
            ));
        frame.render_stateful_widget(file_list, areas.file_list, &mut self.file_list_state);

        // Render PR list panel
        let pr_list_panel = PrListPanel::new(colors)
            .focused(self.focused == FocusedWindow::PrList);
        frame.render_stateful_widget(pr_list_panel, areas.pr_info, &mut self.pr_list_panel_state);

        // Render diff view
        let diff_view = DiffView::new(colors)
            .focused(self.focused == FocusedWindow::Preview);
        frame.render_stateful_widget(diff_view, areas.preview, &mut self.diff_view_state);

        // Render status bar
        self.render_status_bar(frame, areas.status_bar);

        // Render help modal if open
        if self.show_help {
            let help_area = centered_rect(60, 80, area);
            let help = HelpModal::new(colors);
            frame.render_widget(help, help_area);
        }

        // Render input modal if open
        if self.input_modal_state.visible {
            let modal_area = centered_rect(60, 40, area);
            let input_modal = InputModal::new(colors, &self.input_modal_state);
            frame.render_widget(input_modal, modal_area);
        }
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let colors = &self.config.colors;

        let mut spans = vec![];

        // Branch
        spans.push(Span::styled(
            format!(" {} ", self.branch),
            colors.style_status_bar(),
        ));

        // Mode
        spans.push(Span::styled(
            format!(" [{}] ", self.mode),
            colors.style_status_bar(),
        ));

        // File count
        spans.push(Span::styled(
            format!(" {} files ", self.files.len()),
            colors.style_status_bar(),
        ));

        // Diff stats
        if self.mode.is_changed_mode() && (self.diff_stats.added > 0 || self.diff_stats.removed > 0) {
            spans.push(Span::styled(
                format!(" +{} -{} ", self.diff_stats.added, self.diff_stats.removed),
                colors.style_status_bar(),
            ));
        }

        // Pad to fill width
        let content_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let padding = area.width as usize - content_width.min(area.width as usize);
        spans.push(Span::styled(
            " ".repeat(padding),
            colors.style_status_bar(),
        ));

        let line = Line::from(spans);
        frame.render_widget(line, area);
    }
}
