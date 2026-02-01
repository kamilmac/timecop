use anyhow::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    Frame,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::event::KeyInput;
use crate::git::{AppMode, Commit, DiffStats, GitClient, StatusEntry};
use crate::github::{GitHubClient, PrInfo};
use crate::ui::{
    centered_rect, AppLayout, CommitList, CommitListState, DiffView, DiffViewState, FileList,
    FileListState, HelpModal, PreviewContent,
};

/// Which window is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedWindow {
    FileList,
    CommitList,
    Preview,
}

impl FocusedWindow {
    pub fn next(self) -> Self {
        match self {
            Self::FileList => Self::Preview,
            Self::Preview => Self::CommitList,
            Self::CommitList => Self::FileList,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::FileList => Self::CommitList,
            Self::CommitList => Self::Preview,
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

/// Main application state
pub struct App {
    // Core
    pub running: bool,
    pub config: Config,
    git: GitClient,
    github: GitHubClient,

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
    stats_loaded: bool,
    pub pr: Option<PrInfo>,
    last_pr_poll: Instant,

    // Widget states
    pub file_list_state: FileListState,
    pub commit_list_state: CommitListState,
    pub diff_view_state: DiffViewState,
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
            mode: AppMode::default(),
            focused: FocusedWindow::FileList,
            show_help: false,
            pending_command: AppCommand::None,
            branch,
            base_branch,
            files: vec![],
            commits: vec![],
            diff_stats: DiffStats::default(),
            stats_loaded: false,
            pr: None,
            last_pr_poll: Instant::now(),
            file_list_state: FileListState::new(),
            commit_list_state: CommitListState::new(),
            diff_view_state: DiffViewState::new(),
        };

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

        // Diff stats loaded lazily via handle_tick() for faster startup
        self.diff_stats = DiffStats::default();
        self.stats_loaded = false;

        // Update widget states
        self.file_list_state.set_files(self.files.clone());
        self.commit_list_state.set_commits(self.commits.clone());

        // PR info is loaded asynchronously via handle_tick(), not during refresh
        // Clear PR data on branch change so it gets reloaded
        if branch_changed {
            self.pr = None;
        }

        // Update preview
        self.update_preview();

        Ok(())
    }

    /// Refresh PR info from GitHub
    pub fn refresh_pr(&mut self) {
        if !self.github.is_available() {
            return;
        }

        if let Ok(Some(pr)) = self.github.get_pr_for_branch(&self.branch) {
            // Update file comments indicator
            let comments: HashMap<String, bool> = pr
                .file_comments
                .keys()
                .map(|k| (k.clone(), true))
                .collect();
            self.file_list_state.set_comments(comments);

            // Update diff view with PR for inline comments
            self.diff_view_state.set_pr(Some(pr.clone()));

            self.pr = Some(pr);

            // Update preview if showing commit
            self.update_preview();
        }

        self.last_pr_poll = Instant::now();
    }

    /// Handle tick event - periodic updates
    pub fn handle_tick(&mut self) {
        const PR_POLL_INTERVAL: Duration = Duration::from_secs(60);

        // Load diff stats lazily (deferred from refresh for faster startup)
        if self.mode.is_changed_mode() && !self.stats_loaded {
            if let Ok(stats) = self.git.diff_stats(self.mode.diff_mode()) {
                self.diff_stats = stats;
                self.stats_loaded = true;
            }
        }

        // Load PR on first tick (deferred from startup) or on interval
        let should_load = self.pr.is_none() || self.last_pr_poll.elapsed() >= PR_POLL_INTERVAL;
        if should_load && self.github.is_available() {
            self.refresh_pr();
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

        if KeyInput::is_open(&key) {
            self.open_in_editor();
            return Ok(());
        }

        // Window-specific keys
        match self.focused {
            FocusedWindow::FileList => self.handle_file_list_key(&key)?,
            FocusedWindow::CommitList => self.handle_commit_list_key(&key)?,
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

    fn handle_commit_list_key(&mut self, key: &KeyEvent) -> Result<()> {
        let changed = if KeyInput::is_down(key) {
            self.commit_list_state.move_down();
            true
        } else if KeyInput::is_up(key) {
            self.commit_list_state.move_up();
            true
        } else {
            false
        };

        if changed {
            self.update_preview();
        }

        Ok(())
    }

    fn handle_preview_key(&mut self, key: &KeyEvent) -> Result<()> {
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
        if self.focused == FocusedWindow::CommitList {
            // Show commit summary when commit list is focused
            if let Some(commit) = self.commit_list_state.selected().cloned() {
                self.diff_view_state.set_content(PreviewContent::CommitSummary {
                    commit,
                    pr: self.pr.clone(),
                });
            }
        } else {
            self.update_preview();
        }
    }

    fn update_preview(&mut self) {
        if self.focused == FocusedWindow::CommitList {
            if let Some(commit) = self.commit_list_state.selected().cloned() {
                self.diff_view_state.set_content(PreviewContent::CommitSummary {
                    commit,
                    pr: self.pr.clone(),
                });
            }
            return;
        }

        let content = if let Some(entry) = self.file_list_state.selected() {
            if entry.is_root {
                // Root selected - show PR summary or empty
                if let Some(commit) = self.commits.first().cloned() {
                    PreviewContent::CommitSummary {
                        commit,
                        pr: self.pr.clone(),
                    }
                } else {
                    PreviewContent::Empty
                }
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
                // Browse/docs mode - file content
                let content = self.git.read_file(&entry.path).unwrap_or_default();
                PreviewContent::FileContent {
                    path: entry.path.clone(),
                    content,
                }
            } else {
                // Changed mode - diff
                let diff = self
                    .git
                    .diff(&entry.path, self.mode.diff_mode())
                    .unwrap_or_default();
                PreviewContent::FileDiff {
                    path: entry.path.clone(),
                    content: diff,
                }
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

        // Render commit list
        let commit_list = CommitList::new(colors)
            .focused(self.focused == FocusedWindow::CommitList);
        frame.render_stateful_widget(commit_list, areas.commit_list, &mut self.commit_list_state);

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
