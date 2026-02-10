use anyhow::Result;
use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    Frame,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::async_loader::AsyncLoader;
use crate::config::Config;
use crate::event::KeyInput;
use crate::git::{DiffStats, GitClient, TimelinePosition};
use crate::github::{GitHubClient, PrInfo};
use crate::ui::{
    centered_rect, Action, AppLayout, DiffView, DiffViewState, FileList, FileListState, HelpModal,
    Highlighter, InputModal, InputModalState, InputResult, LayoutAreas, PrDetailsView,
    PrDetailsViewState, PrListPanel, PrListPanelState, PreviewContent, ReviewAction,
};

/// Which window is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedWindow {
    FileList,
    PrList,
    Preview,
}

impl FocusedWindow {
    /// Tab cycles clockwise: Files → Preview → PRs
    pub fn next(self) -> Self {
        match self {
            Self::FileList => Self::Preview,
            Self::Preview => Self::PrList,
            Self::PrList => Self::FileList,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::FileList => Self::PrList,
            Self::PrList => Self::Preview,
            Self::Preview => Self::FileList,
        }
    }
}

/// Command to execute after handling input
#[derive(Debug, Clone)]
pub enum AppCommand {
    None,
    OpenEditor { path: String, line: Option<usize> },
}

/// Toast notification for temporary messages
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    pub created_at: Instant,
}

impl Toast {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: false,
            created_at: Instant::now(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_error: true,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > Duration::from_secs(3)
    }
}

/// Main application state
pub struct App {
    // Core
    pub running: bool,
    pub config: Config,
    git: GitClient,
    github: GitHubClient,

    // State
    pub focused: FocusedWindow,
    pub show_help: bool,
    pub pending_command: AppCommand,
    pub timeline_position: TimelinePosition,
    pub commit_count: usize,

    // Data
    pub branch: String,
    pub diff_stats: DiffStats,
    pub selected_pr: Option<PrInfo>,

    // Notifications
    pub toast: Option<Toast>,
    pub gh_available: bool,
    pub tick_count: usize,

    // Async loading
    async_loader: AsyncLoader,
    last_pr_list_poll: Instant,

    // Widget states
    pub file_list_state: FileListState,
    pub pr_list_panel_state: PrListPanelState,
    pub diff_view_state: DiffViewState,
    pub pr_details_view_state: PrDetailsViewState,
    pub input_modal_state: InputModalState,

    // Syntax highlighting
    highlighter: Highlighter,

    // Layout areas for mouse hit testing
    layout_areas: Option<LayoutAreas>,

}

impl App {
    pub fn new(path: &str) -> Result<Self> {
        let git = GitClient::open(path)?;
        let mut github = GitHubClient::new();

        // Check gh CLI availability upfront
        let gh_available = github.is_available();

        let branch = git.current_branch().unwrap_or_else(|_| "HEAD".to_string());

        let config = Config::default();
        let pr_poll_interval = config.timing.pr_poll_interval;
        let highlighter = Highlighter::for_theme(config.theme);
        let mut app = Self {
            running: true,
            git,
            github,
            focused: FocusedWindow::FileList,
            show_help: false,
            pending_command: AppCommand::None,
            timeline_position: TimelinePosition::default(),
            commit_count: 0,
            branch,
            diff_stats: DiffStats::default(),
            selected_pr: None,
            toast: None,
            gh_available,
            tick_count: 0,
            async_loader: AsyncLoader::new(),
            last_pr_list_poll: Instant::now() - pr_poll_interval - Duration::from_secs(1), // Force immediate load
            file_list_state: FileListState::new(),
            pr_list_panel_state: PrListPanelState::new(),
            diff_view_state: DiffViewState::new(),
            pr_details_view_state: PrDetailsViewState::new(),
            input_modal_state: InputModalState::new(),
            highlighter,
            config,
            layout_areas: None,
        };

        // Initialize PR list panel
        app.pr_list_panel_state.set_gh_available(gh_available);
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

        // Re-detect and fetch base branch (keeps origin/main etc. up to date)
        if branch_changed {
            self.git.refresh_base_branch();
        } else {
            self.git.fetch_base_branch();
        }

        // Update commit count for timeline
        self.commit_count = self.git.commit_count_since_base().unwrap_or(0);

        // Load files based on timeline position
        let files = self.git.status_at_position(self.timeline_position)?;
        self.file_list_state.set_files(files);

        // Auto-select first file if cursor is at root and there are files
        // (Skip root "./" entry at index 0, select first actual file at index 1+)
        if self.file_list_state.scroll.cursor == 0 && self.file_list_state.entries.len() > 1 {
            // Find first non-directory entry, or first entry after root
            let first_file_idx = self.file_list_state.entries.iter()
                .position(|e| !e.is_dir && !e.is_root)
                .unwrap_or(1);
            self.file_list_state.scroll.cursor = first_file_idx.min(self.file_list_state.entries.len() - 1);
        }

        // Compute full diff stats on branch change or if not yet calculated
        if branch_changed || (self.diff_stats.added == 0 && self.diff_stats.removed == 0) {
            self.diff_stats = self.git.diff_stats_at_position(TimelinePosition::FullDiff)
                .unwrap_or_default();
        }

        // Update PR list panel with current branch
        if branch_changed {
            self.pr_list_panel_state.set_current_branch(self.branch.clone());
            // Clear selected PR details since branch changed
            self.selected_pr = None;
            // Reset timeline to default when branch changes
            self.timeline_position = TimelinePosition::default();
        }

        // Force PR list reload on manual refresh
        self.last_pr_list_poll = Instant::now() - self.config.timing.pr_poll_interval - Duration::from_secs(1);

        // Update preview
        self.update_preview();

        Ok(())
    }

    /// Switch to a new timeline position, preserving view state
    fn switch_timeline(&mut self, new_position: TimelinePosition) -> Result<()> {
        if new_position == self.timeline_position {
            return Ok(());
        }

        self.diff_view_state.save_line_position();
        self.file_list_state.save_selected_path();

        let leaving_browse = matches!(self.timeline_position, TimelinePosition::Browse);
        let entering_browse = matches!(new_position, TimelinePosition::Browse);

        self.file_list_state.save_mode_state(leaving_browse);
        self.file_list_state.restore_mode_state(entering_browse);

        self.timeline_position = new_position;

        let files = self.git.status_at_position(self.timeline_position)?;
        self.file_list_state.set_files(files);

        if entering_browse {
            self.file_list_state.initialize_browse_mode();
        }

        self.file_list_state.restore_selection();
        self.update_preview();

        Ok(())
    }

    /// Load PR details for a specific PR number
    fn load_pr_details(&mut self, pr_number: u64) {
        let already_loaded = self.selected_pr.as_ref().map(|p| p.number) == Some(pr_number);
        let already_loading = self.async_loader.loading_pr_number() == Some(pr_number);

        if !already_loaded && !already_loading {
            self.async_loader.load_pr_details(pr_number);
        }
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
        self.tick_count = self.tick_count.wrapping_add(1);

        // Clear expired toasts
        if let Some(ref toast) = self.toast {
            if toast.is_expired() {
                self.toast = None;
            }
        }

        let pr_poll_interval = self.config.timing.pr_poll_interval;

        // Poll for completed PR list loading
        if let Some(prs) = self.async_loader.poll_pr_list() {
            self.pr_list_panel_state.set_prs(prs);
            self.last_pr_list_poll = Instant::now();

            // Auto-load details for selected PR
            if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                self.load_pr_details(pr_num);
            }
        }

        // Poll for completed PR detail loading
        if let Some((pr_number, pr_opt)) = self.async_loader.poll_pr_details() {
            if let Some(pr) = pr_opt {
                // Only apply if this PR is still the one we want
                let currently_selected = self.pr_list_panel_state.selected_number();
                if currently_selected == Some(pr_number) {
                    self.apply_pr_details(pr);
                }
            }
            // Update preview to show loaded content or clear loading state
            if self.focused == FocusedWindow::PrList {
                self.show_selected_pr_in_preview();
            }
        }

        // Update loading state in PR panel
        self.pr_list_panel_state.loading = self.async_loader.is_pr_list_loading();

        // Trigger PR list loading if needed (on startup and periodically)
        // Skip if gh CLI is not available
        if self.gh_available {
            let should_load_pr_list = self.last_pr_list_poll.elapsed() >= pr_poll_interval;
            if should_load_pr_list && !self.async_loader.is_pr_list_loading() {
                self.async_loader.load_pr_list();
            }
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

        // Toggle diff view mode (global - works from any pane)
        if KeyInput::is_toggle_view_mode(&key) {
            self.diff_view_state.toggle_view_mode();
            return Ok(());
        }

        // Tab cycles through all panes
        if KeyInput::is_tab(&key) {
            self.focused = self.focused.next();
            self.on_focus_change();
            return Ok(());
        }

        if KeyInput::is_shift_tab(&key) {
            self.focused = self.focused.prev();
            self.on_focus_change();
            return Ok(());
        }

        // Enter/Space is context-sensitive
        if KeyInput::is_select(&key) {
            match self.focused {
                FocusedWindow::FileList => {
                    // Go to preview
                    self.focused = FocusedWindow::Preview;
                    self.on_focus_change();
                }
                FocusedWindow::PrList => {
                    // Checkout PR branch directly
                    if let Some(pr) = self.pr_list_panel_state.selected() {
                        self.checkout_pr(pr.number)?;
                    }
                }
                FocusedWindow::Preview => {}
            }
            return Ok(());
        }

        // Escape goes back to left pane
        if KeyInput::is_escape(&key) && self.focused == FocusedWindow::Preview {
            // Go back to PrList if viewing PR details, otherwise FileList
            if self.pr_details_view_state.pr.is_some() || self.pr_details_view_state.loading_message.is_some() {
                self.focused = FocusedWindow::PrList;
            } else {
                self.focused = FocusedWindow::FileList;
            }
            self.on_focus_change();
            return Ok(());
        }

        if KeyInput::is_yank(&key) {
            self.yank_path();
            return Ok(());
        }

        // Timeline navigation: , goes left (older), . goes right (newer)
        if KeyInput::is_timeline_next(&key) {
            // , key - go older (left on timeline)
            let new_pos = self.timeline_position.prev(self.commit_count);
            self.switch_timeline(new_pos)?;
            return Ok(());
        }
        if KeyInput::is_timeline_prev(&key) {
            // . key - go newer (right on timeline)
            let new_pos = self.timeline_position.next();
            self.switch_timeline(new_pos)?;
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

        if KeyInput::is_open_preview(&key) {
            self.open_preview();
            return Ok(());
        }

        // Window-specific keys - delegate to widget, dispatch action
        let action = match self.focused {
            FocusedWindow::FileList => self.file_list_state.handle_key(&key),
            FocusedWindow::PrList => self.pr_list_panel_state.handle_key(&key),
            FocusedWindow::Preview => {
                // Check if we're in PR details context or file diff context
                if self.pr_details_view_state.pr.is_some() || self.pr_details_view_state.loading_message.is_some() {
                    self.pr_details_view_state.handle_key(&key)
                } else {
                    let pr_number = self.pr_list_panel_state.selected_number();
                    self.diff_view_state.handle_key(&key, pr_number)
                }
            }
        };

        self.dispatch(action)?;

        Ok(())
    }

    /// Handle mouse input
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Result<()> {
        if self.input_modal_state.visible || self.show_help {
            return Ok(());
        }
        let Some(areas) = self.layout_areas.clone() else { return Ok(()) };
        let (x, y) = (mouse.column, mouse.row);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_click(x, y, &areas),
            MouseEventKind::ScrollDown => self.handle_scroll(true),
            MouseEventKind::ScrollUp => self.handle_scroll(false),
            _ => {}
        }
        Ok(())
    }

    fn handle_click(&mut self, x: u16, y: u16, areas: &LayoutAreas) {
        let target = if areas.file_list.intersects(Rect::new(x, y, 1, 1)) {
            Some((FocusedWindow::FileList, y.saturating_sub(areas.file_list.y + 1)))
        } else if areas.pr_info.intersects(Rect::new(x, y, 1, 1)) {
            Some((FocusedWindow::PrList, y.saturating_sub(areas.pr_info.y + 1)))
        } else if areas.preview.intersects(Rect::new(x, y, 1, 1)) {
            Some((FocusedWindow::Preview, y.saturating_sub(areas.preview.y + 1)))
        } else {
            None
        };

        if let Some((window, row)) = target {
            if self.focused != window {
                self.focused = window;
                self.on_focus_change();
            }
            match window {
                FocusedWindow::FileList => {
                    self.file_list_state.scroll.click_at(row as usize);
                    self.update_preview();
                }
                FocusedWindow::PrList => {
                    if self.pr_list_panel_state.click_at(row as usize) {
                        if let Some(n) = self.pr_list_panel_state.selected_number() {
                            self.load_pr_details(n);
                            self.show_selected_pr_in_preview();
                        }
                    }
                }
                FocusedWindow::Preview => {
                    if self.pr_details_view_state.pr.is_some() {
                        self.pr_details_view_state.scroll.click_at(row as usize);
                    } else {
                        self.diff_view_state.scroll.click_at(row as usize);
                    }
                }
            }
        }
    }

    fn handle_scroll(&mut self, down: bool) {
        match self.focused {
            FocusedWindow::FileList => {
                if down { self.file_list_state.scroll.move_down_n(3) } else { self.file_list_state.scroll.move_up_n(3) }
                self.update_preview();
            }
            FocusedWindow::PrList => {
                if down { self.pr_list_panel_state.scroll.move_down() } else { self.pr_list_panel_state.scroll.move_up() }
                if let Some(n) = self.pr_list_panel_state.selected_number() {
                    self.load_pr_details(n);
                    self.show_selected_pr_in_preview();
                }
            }
            FocusedWindow::Preview => {
                if self.pr_details_view_state.pr.is_some() {
                    if down { self.pr_details_view_state.scroll.move_down_n(3) } else { self.pr_details_view_state.scroll.move_up_n(3) }
                } else {
                    if down { self.diff_view_state.scroll.move_down_n(3) } else { self.diff_view_state.scroll.move_up_n(3) }
                }
            }
        }
    }

    /// Dispatch an action from a widget
    fn dispatch(&mut self, action: Action) -> Result<()> {
        match action {
            Action::None | Action::Ignored => {}

            Action::FileSelected(_path) => {
                self.focused = FocusedWindow::Preview;
                self.on_focus_change();
            }

            Action::PrSelected(pr_number) => {
                self.load_pr_details(pr_number);
                self.show_selected_pr_in_preview();
            }

            Action::CheckoutPr(pr_number) => {
                self.checkout_pr(pr_number)?;
            }

            Action::ExpandIgnoredDir(dir_path) => {
                if let Ok(entries) = self.git.list_ignored_dir(&dir_path) {
                    self.file_list_state.insert_ignored_dir_contents(&dir_path, entries);
                }
            }

            Action::OpenReviewModal(review_action) => {
                self.input_modal_state.show(review_action);
            }
        }

        // Update preview after actions that change file list state
        if matches!(self.focused, FocusedWindow::FileList) {
            self.update_preview();
        }

        Ok(())
    }

    fn show_selected_pr_in_preview(&mut self) {
        // Show loading indicator if fetching PR details
        if self.async_loader.is_pr_detail_loading() {
            if let Some(pr) = self.pr_list_panel_state.selected() {
                self.pr_details_view_state.set_loading(
                    format!("Loading PR #{} details...", pr.number),
                );
            }
            return;
        }

        // Show selected PR details in preview
        if let Some(pr) = self.selected_pr.clone() {
            self.pr_details_view_state.set_pr(Some(pr));
        } else if let Some(summary) = self.pr_list_panel_state.selected() {
            // Show basic info from summary if full details not loaded yet
            self.pr_details_view_state.set_loading(
                format!("PR #{}: {}", summary.number, summary.title),
            );
        } else {
            self.pr_details_view_state.clear();
        }
    }

    fn on_focus_change(&mut self) {
        if self.focused == FocusedWindow::PrList {
            // Show selected PR in preview when PR list is focused
            self.show_selected_pr_in_preview();
            // Load details if needed
            if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                self.load_pr_details(pr_num);
            }
        } else if self.focused == FocusedWindow::Preview {
            // Keep PR details when focusing preview (allows scrolling PR details)
            // Only update diff preview if not viewing PR details
            if self.pr_details_view_state.pr.is_none() && self.pr_details_view_state.loading_message.is_none() {
                self.update_preview();
            }
        } else {
            // FileList focused - clear PR details and show file diff
            self.pr_details_view_state.clear();
            self.update_preview();
        }
    }

    fn update_preview(&mut self) {
        if self.focused == FocusedWindow::PrList {
            self.show_selected_pr_in_preview();
            return;
        }

        // Save current file's line position before switching
        self.diff_view_state.save_line_position();

        let is_browse_mode = matches!(self.timeline_position, TimelinePosition::Browse);

        let content = if let Some(entry) = self.file_list_state.selected() {
            if entry.is_root {
                PreviewContent::Empty
            } else if entry.is_dir {
                if is_browse_mode {
                    // In browse mode, directories don't have a combined view
                    PreviewContent::Empty
                } else {
                    // Directory selected - combined diff at timeline position
                    let diff = self
                        .git
                        .diff_files_at_position(&entry.children, self.timeline_position)
                        .unwrap_or_default();
                    PreviewContent::FolderDiff {
                        path: entry.path.clone(),
                        content: diff,
                    }
                }
            } else if is_browse_mode {
                // Browse mode - show file content
                let file_content = self
                    .git
                    .diff_at_position(&entry.path, self.timeline_position)
                    .unwrap_or_default();
                let content = PreviewContent::FileContent {
                    path: entry.path.clone(),
                    content: file_content,
                };
                self.diff_view_state.set_content_highlighted(content, &self.highlighter);
                self.diff_view_state.restore_line_position();
                return;
            } else {
                // File selected - diff with syntax highlighting at timeline position
                let diff = self
                    .git
                    .diff_at_position(&entry.path, self.timeline_position)
                    .unwrap_or_default();
                let content = PreviewContent::FileDiff {
                    path: entry.path.clone(),
                    content: diff,
                };
                self.diff_view_state.set_content_highlighted(content, &self.highlighter);
                self.diff_view_state.restore_line_position();
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

    fn open_preview(&self) {
        let entry = match self.file_list_state.selected() {
            Some(e) if !e.is_dir => e,
            _ => return,
        };

        let path = &entry.path;
        let full_path = self.git.path().join(path);

        // read_to_string fails on binary files (invalid UTF-8), which is what we want
        let content = match std::fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let is_markdown = path.ends_with(".md") || path.ends_with(".markdown");
        let body = if is_markdown {
            content
        } else {
            let ext = std::path::Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            format!("```{}\n{}\n```", ext, content)
        };

        let compressed = lz_str::compress_to_encoded_uri_component(&body);
        let url = format!("https://kamilmac.github.io/mdash/#{}", compressed);

        let _ = std::process::Command::new("open")
            .arg(&url)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    /// Take pending command (clears it)
    pub fn take_command(&mut self) -> AppCommand {
        std::mem::replace(&mut self.pending_command, AppCommand::None)
    }

    /// Checkout a PR branch directly
    fn checkout_pr(&mut self, pr_number: u64) -> Result<()> {
        // Block checkout if there are uncommitted changes
        if self.git.has_uncommitted_changes() {
            self.toast = Some(Toast::error("Commit or stash changes before switching branches"));
            return Ok(());
        }

        match self.github.checkout_pr(pr_number) {
            Ok(base_branch) => {
                if !base_branch.is_empty() {
                    self.git.set_base_branch(&base_branch);
                }
                self.toast = Some(Toast::success("Switched to PR branch"));
                self.refresh()?;
            }
            Err(e) => {
                self.toast = Some(Toast::error(format!("Checkout failed: {}", e)));
            }
        }
        Ok(())
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
            ReviewAction::ReplyToComment { pr_number, comment_id, .. } => {
                self.github.reply_to_comment(*pr_number, *comment_id, &body)
            }
        };

        match result {
            Ok(()) => {
                self.input_modal_state.hide();

                // Show success toast
                let success_msg = match &action {
                    ReviewAction::Approve { .. } => "PR approved",
                    ReviewAction::RequestChanges { .. } => "Changes requested",
                    ReviewAction::Comment { .. } => "Comment posted",
                    ReviewAction::LineComment { .. } => "Line comment added",
                    ReviewAction::ReplyToComment { .. } => "Reply posted",
                };
                self.toast = Some(Toast::success(success_msg));

                // Refresh PR details to show the new comment/review
                if let Some(pr_num) = self.pr_list_panel_state.selected_number() {
                    // Force reload by clearing the current PR
                    self.selected_pr = None;
                    self.async_loader.load_pr_details(pr_num);
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
        let areas = layout.compute(area, self.pr_list_panel_state.prs.len());

        // Store layout areas for mouse hit testing
        self.layout_areas = Some(areas.clone());

        // Render header with app name
        self.render_header(frame, areas.header);

        // Render file list with context-aware title
        let file_list_title = self.file_list_title();
        let file_list = FileList::new(colors)
            .focused(self.focused == FocusedWindow::FileList)
            .title(file_list_title);
        frame.render_stateful_widget(file_list, areas.file_list, &mut self.file_list_state);

        // Render PR list panel
        let pr_list_panel = PrListPanel::new(colors)
            .focused(self.focused == FocusedWindow::PrList)
            .spinner_frame(self.tick_count / 2); // Slow down spinner
        frame.render_stateful_widget(pr_list_panel, areas.pr_info, &mut self.pr_list_panel_state);

        // Render preview: PR details view or diff view depending on context
        let preview_focused = self.focused == FocusedWindow::Preview;
        if self.pr_details_view_state.pr.is_some() || self.pr_details_view_state.loading_message.is_some() {
            let pr_details_view = PrDetailsView::new(colors).focused(preview_focused);
            frame.render_stateful_widget(pr_details_view, areas.preview, &mut self.pr_details_view_state);
        } else {
            // Auto-adjust diff view mode based on preview width
            self.diff_view_state.auto_adjust_view_mode(areas.preview.width);
            let diff_view = DiffView::new(colors).focused(preview_focused);
            frame.render_stateful_widget(diff_view, areas.preview, &mut self.diff_view_state);
        }

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
            // Use smaller modal for confirmations, larger for text input
            let height = if self.input_modal_state.action.as_ref().map(|a| a.needs_body()).unwrap_or(false) {
                40 // Text input needs more space
            } else {
                15 // Confirmation dialogs are compact
            };
            let modal_area = centered_rect(50, height, area);
            let input_modal = InputModal::new(colors, &self.input_modal_state);
            frame.render_widget(input_modal, modal_area);
        }

        // Render toast notification
        if let Some(ref toast) = self.toast {
            self.render_toast(frame, area, toast);
        }
    }

    fn render_toast(&self, frame: &mut Frame, area: Rect, toast: &Toast) {
        use ratatui::style::Color;

        let msg_len = toast.message.chars().count() as u16 + 4; // padding
        let toast_width = msg_len.min(area.width.saturating_sub(4));
        let toast_x = area.width.saturating_sub(toast_width + 2);
        let toast_y = area.height.saturating_sub(3);

        let toast_area = Rect::new(toast_x, toast_y, toast_width, 1);

        let (bg, fg) = if toast.is_error {
            (Color::Rgb(180, 60, 60), Color::White)
        } else {
            (Color::Rgb(60, 140, 80), Color::White)
        };

        let style = ratatui::style::Style::default().bg(bg).fg(fg);
        let text = format!(" {} ", toast.message);
        let line = Line::from(Span::styled(text, style));
        frame.render_widget(line, toast_area);
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        use ratatui::style::Modifier;

        let total_width = area.width as usize;
        let colors = &self.config.colors;

        let primary_bold = ratatui::style::Style::default()
            .fg(colors.logo_primary)
            .add_modifier(Modifier::BOLD);
        let highlight_bold = ratatui::style::Style::default()
            .fg(colors.logo_highlight)
            .add_modifier(Modifier::BOLD);
        let dim_style = ratatui::style::Style::default()
            .fg(colors.muted);

        // Timeline layout (left to right, older to newer):
        // T─I─M─E─C─O─P─○─○─○─●─[full]─[files]
        //               -3-2-1 wip full  files

        let mut spans = Vec::new();

        // Left padding
        spans.push(Span::raw(" "));

        // TIMECOP logo
        let logo = ["T", "─", "I", "─", "M", "─", "E", "─", "C", "─", "O", "─", "P"];
        for elem in logo.iter() {
            spans.push(Span::styled(*elem, primary_bold));
        }

        // Separator (only if there are commits or wip)
        let num_commits = self.commit_count.min(16);
        spans.push(Span::styled("─", primary_bold));

        // Commit dots (only show available commits, max 16) - oldest first
        for i in (1..=num_commits).rev() {
            let is_selected = matches!(self.timeline_position, TimelinePosition::CommitDiff(n) if n == i);
            let style = if is_selected { highlight_bold } else { primary_bold };
            spans.push(Span::styled("○", style));
            spans.push(Span::styled("─", if is_selected { highlight_bold } else { primary_bold }));
        }

        // Wip marker (● filled dot - uncommitted changes, like a commit in progress)
        let wip_selected = matches!(self.timeline_position, TimelinePosition::Wip);
        let wip_style = if wip_selected { highlight_bold } else { primary_bold };
        spans.push(Span::styled("●", wip_style));
        spans.push(Span::styled("─", if wip_selected { highlight_bold } else { primary_bold }));

        // [full] marker (full diff - all committed changes)
        let full_selected = matches!(self.timeline_position, TimelinePosition::FullDiff);
        spans.push(Span::styled("[", primary_bold));
        spans.push(Span::styled("full", if full_selected { highlight_bold } else { primary_bold }));
        spans.push(Span::styled("]", primary_bold));
        spans.push(Span::styled("─", primary_bold));

        // [files] marker (browse all files)
        let files_selected = matches!(self.timeline_position, TimelinePosition::Browse);
        spans.push(Span::styled("[", primary_bold));
        spans.push(Span::styled("files", if files_selected { highlight_bold } else { primary_bold }));
        spans.push(Span::styled("]", primary_bold));

        // State label
        let state_label = match self.timeline_position {
            TimelinePosition::Browse => "files",
            TimelinePosition::Wip => "wip",
            TimelinePosition::FullDiff => "full diff",
            TimelinePosition::CommitDiff(n) => match n {
                1 => "-1", 2 => "-2", 3 => "-3", 4 => "-4", 5 => "-5",
                6 => "-6", 7 => "-7", 8 => "-8", 9 => "-9", 10 => "-10",
                11 => "-11", 12 => "-12", 13 => "-13", 14 => "-14", 15 => "-15",
                _ => "-16",
            },
        };
        const LABEL_WIDTH: usize = 11;
        let padded_label = format!("  {:width$}", state_label, width = LABEL_WIDTH);
        spans.push(Span::styled(padded_label, dim_style));

        // Help hint on the right
        let help_hint = "? help";
        let content_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        let right_pad = total_width.saturating_sub(content_width + help_hint.len() + 2);
        spans.push(Span::raw(" ".repeat(right_pad)));
        spans.push(Span::styled(format!(" {} ", help_hint), dim_style));

        frame.render_widget(Line::from(spans), area);
    }

    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let colors = &self.config.colors;
        let total_width = area.width as usize;

        // Left: branch (+stats in full diff mode)
        let left_content = if matches!(self.timeline_position, TimelinePosition::FullDiff)
            && (self.diff_stats.added > 0 || self.diff_stats.removed > 0) {
            format!(" {}  +{} -{}", self.branch, format_count(self.diff_stats.added), format_count(self.diff_stats.removed))
        } else {
            format!(" {}", self.branch)
        };

        // Right: position info
        let right_content = match self.timeline_position {
            TimelinePosition::Browse => "all files ".to_string(),
            TimelinePosition::FullDiff => "full diff (base → head) ".to_string(),
            TimelinePosition::Wip => "uncommitted (wip) ".to_string(),
            TimelinePosition::CommitDiff(n) => {
                if let Some(msg) = self.timeline_commit_message() {
                    let max_len = 40;
                    let truncated = if msg.len() > max_len {
                        format!("{}...", &msg[..max_len])
                    } else {
                        msg
                    };
                    format!("-{}: {} ", n, truncated)
                } else {
                    format!("-{} ", n)
                }
            }
        };

        let left_width = left_content.chars().count();
        let right_width = right_content.chars().count();
        let padding = total_width.saturating_sub(left_width + right_width);

        let line = Line::from(vec![
            Span::styled(left_content, colors.style_status_bar()),
            Span::styled(" ".repeat(padding), colors.style_status_bar()),
            Span::styled(right_content, colors.style_status_bar()),
        ]);
        frame.render_widget(line, area);
    }

    /// Generate file list title
    fn file_list_title(&self) -> String {
        if matches!(self.timeline_position, TimelinePosition::Browse) {
            format!("All Files ({})", self.file_list_state.file_count())
        } else {
            format!("Changed ({})", self.file_list_state.file_count())
        }
    }

    /// Get commit message for current timeline position (for status bar)
    fn timeline_commit_message(&self) -> Option<String> {
        match self.timeline_position {
            TimelinePosition::CommitDiff(n) => {
                self.git.commit_summary_at_offset(n - 1).ok()
            }
            _ => None,
        }
    }
}

/// Format large numbers with K/M suffixes
fn format_count(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 10_000 {
        format!("{}K", n / 1000)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}
