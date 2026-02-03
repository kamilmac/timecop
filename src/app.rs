use anyhow::Result;
use crossterm::event::KeyEvent;
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
use crate::git::{DiffStats, GitClient, StatusEntry, TimelinePosition};
use crate::github::{GitHubClient, PrInfo};
use crate::ui::{
    centered_rect, Action, AppLayout, DiffView, DiffViewState, FileList, FileListState, HelpModal,
    Highlighter, InputModal, InputModalState, InputResult, PrDetailsView, PrDetailsViewState,
    PrListPanel, PrListPanelState, PreviewContent, ReviewAction, ReviewActionType,
};

/// Which window is focused
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedWindow {
    FileList,
    PrList,
    Preview,
}

impl FocusedWindow {
    /// Tab cycles through all panes
    pub fn next(self) -> Self {
        match self {
            Self::FileList => Self::PrList,
            Self::PrList => Self::Preview,
            Self::Preview => Self::FileList,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::FileList => Self::Preview,
            Self::PrList => Self::FileList,
            Self::Preview => Self::PrList,
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
    pub focused: FocusedWindow,
    pub show_help: bool,
    pub pending_command: AppCommand,
    pub timeline_position: TimelinePosition,
    pub commit_count: usize,

    // Data
    pub branch: String,
    pub files: Vec<StatusEntry>,
    pub diff_stats: DiffStats,
    pub selected_pr: Option<PrInfo>,

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
}

impl App {
    pub fn new(path: &str) -> Result<Self> {
        let git = GitClient::open(path)?;
        let github = GitHubClient::new();

        let branch = git.current_branch().unwrap_or_else(|_| "HEAD".to_string());

        let pr_poll_interval = Config::default().timing.pr_poll_interval;
        let mut app = Self {
            running: true,
            config: Config::default(),
            git,
            github,
            focused: FocusedWindow::FileList,
            show_help: false,
            pending_command: AppCommand::None,
            timeline_position: TimelinePosition::default(),
            commit_count: 0,
            branch,
            files: vec![],
            diff_stats: DiffStats::default(),
            selected_pr: None,
            async_loader: AsyncLoader::new(),
            last_pr_list_poll: Instant::now() - pr_poll_interval - Duration::from_secs(1), // Force immediate load
            file_list_state: FileListState::new(),
            pr_list_panel_state: PrListPanelState::new(),
            diff_view_state: DiffViewState::new(),
            pr_details_view_state: PrDetailsViewState::new(),
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

        // Update commit count for timeline
        self.commit_count = self.git.commit_count_since_base().unwrap_or(0);

        // Load files based on timeline position
        self.files = self.git.status_at_position(self.timeline_position)?;

        // Compute full diff stats only on branch change (not on timeline change)
        if branch_changed || self.diff_stats.added == 0 && self.diff_stats.removed == 0 {
            self.diff_stats = self.git.diff_stats_at_position(TimelinePosition::FullDiff)
                .unwrap_or_default();
        }

        // Update widget states
        self.file_list_state.set_files(self.files.clone());

        // Update PR list panel with current branch
        if branch_changed {
            self.pr_list_panel_state.set_current_branch(self.branch.clone());
            // Clear selected PR details since branch changed
            self.selected_pr = None;
            // Reset timeline to default when branch changes
            self.timeline_position = TimelinePosition::default();
        }

        // Update preview
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
        let should_load_pr_list = self.last_pr_list_poll.elapsed() >= pr_poll_interval;
        if should_load_pr_list && !self.async_loader.is_pr_list_loading() {
            self.async_loader.load_pr_list();
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

        if KeyInput::is_yank(&key) {
            self.yank_path();
            return Ok(());
        }

        // Timeline navigation: , goes to next (older), . goes to prev (newer)
        if KeyInput::is_timeline_next(&key) {
            self.timeline_position = self.timeline_position.next(self.commit_count);
            self.refresh()?;
            return Ok(());
        }
        if KeyInput::is_timeline_prev(&key) {
            self.timeline_position = self.timeline_position.prev();
            self.refresh()?;
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

    /// Dispatch an action from a widget
    fn dispatch(&mut self, action: Action) -> Result<()> {
        match action {
            Action::None | Action::Ignored => {}

            Action::Quit => {
                self.running = false;
            }

            Action::Refresh => {
                self.refresh()?;
            }

            Action::ToggleHelp => {
                self.show_help = !self.show_help;
            }

            Action::ChangeFocus(target) => {
                use crate::ui::FocusTarget;
                self.focused = match target {
                    FocusTarget::FileList => FocusedWindow::FileList,
                    FocusTarget::PrList => FocusedWindow::PrList,
                    FocusTarget::Preview => FocusedWindow::Preview,
                    FocusTarget::Next => self.focused.next(),
                    FocusTarget::Prev => self.focused.prev(),
                };
                self.on_focus_change();
            }

            Action::FileSelected(_path) => {
                // Go to preview when file is selected
                self.focused = FocusedWindow::Preview;
                self.on_focus_change();
            }

            Action::PrSelected(pr_number) => {
                self.load_pr_details(pr_number);
                self.show_selected_pr_in_preview();
            }

            Action::CheckoutPr(pr_number) => {
                let _ = self.github.checkout_pr(pr_number);
                self.refresh()?;
            }

            Action::OpenPrInBrowser(pr_number) => {
                let _ = self.github.open_pr_in_browser(pr_number);
            }

            Action::OpenReviewModal(review_type) => {
                let review_action = match review_type {
                    ReviewActionType::Approve { pr_number } => {
                        ReviewAction::Approve { pr_number }
                    }
                    ReviewActionType::RequestChanges { pr_number } => {
                        ReviewAction::RequestChanges { pr_number }
                    }
                    ReviewActionType::Comment { pr_number } => {
                        ReviewAction::Comment { pr_number }
                    }
                    ReviewActionType::LineComment { pr_number, path, line } => {
                        ReviewAction::LineComment { pr_number, path, line }
                    }
                };
                self.input_modal_state.show(review_action);
            }

            Action::YankPath => {
                self.yank_path();
            }

            Action::OpenInEditor => {
                self.open_in_editor();
            }

            Action::TimelineNext => {
                self.timeline_position = self.timeline_position.next(self.commit_count);
                self.refresh()?;
            }

            Action::TimelinePrev => {
                self.timeline_position = self.timeline_position.prev();
                self.refresh()?;
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
        } else {
            // Clear PR details view when leaving PR context
            self.pr_details_view_state.clear();
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
                PreviewContent::Empty
            } else if entry.is_dir {
                // Directory selected - combined diff
                let diff = self
                    .git
                    .diff_files(&entry.children)
                    .unwrap_or_default();
                PreviewContent::FolderDiff {
                    path: entry.path.clone(),
                    content: diff,
                }
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
            .focused(self.focused == FocusedWindow::PrList);
        frame.render_stateful_widget(pr_list_panel, areas.pr_info, &mut self.pr_list_panel_state);

        // Render preview: PR details view or diff view depending on context
        let preview_focused = self.focused == FocusedWindow::Preview;
        if self.pr_details_view_state.pr.is_some() || self.pr_details_view_state.loading_message.is_some() {
            let pr_details_view = PrDetailsView::new(colors).focused(preview_focused);
            frame.render_stateful_widget(pr_details_view, areas.preview, &mut self.pr_details_view_state);
        } else {
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
            let modal_area = centered_rect(60, 40, area);
            let input_modal = InputModal::new(colors, &self.input_modal_state);
            frame.render_widget(input_modal, modal_area);
        }
    }

    fn render_header(&self, frame: &mut Frame, area: Rect) {
        use ratatui::style::{Color, Modifier};

        let total_width = area.width as usize;
        let timecop_green = Color::Rgb(150, 255, 170);
        let timecop_red = Color::Rgb(255, 80, 80);

        let green_bold = ratatui::style::Style::default()
            .fg(timecop_green)
            .add_modifier(Modifier::BOLD);
        let red_bold = ratatui::style::Style::default()
            .fg(timecop_red)
            .add_modifier(Modifier::BOLD);

        // TIMECOP as timeline indicator
        // Elements: ◆─T─I─M─E─C─O─P─◆ (9 positions)
        // Position mapping (right to left): 0=◆, 1=P, 2=O, 3=C, 4=E, 5=M, 6=I, 7=T, 8=◆
        let elements = ["◆", "─", "T", "─", "I", "─", "M", "─", "E", "─", "C", "─", "O", "─", "P", "─", "◆"];
        let position_to_index = [16, 14, 12, 10, 8, 6, 4, 2, 0]; // maps timeline position to element index

        let selected_idx = self.timeline_position.display_index().min(8);
        let highlight_center = position_to_index[selected_idx];

        let mut spans = Vec::new();
        for (i, elem) in elements.iter().enumerate() {
            // Highlight the selected element and adjacent dashes
            let is_highlighted = (i as isize - highlight_center as isize).abs() <= 1;
            let style = if is_highlighted { red_bold } else { green_bold };
            spans.push(Span::styled(*elem, style));
        }

        // Center the logo
        let logo_width = elements.iter().map(|s| s.chars().count()).sum::<usize>();
        let left_pad = total_width.saturating_sub(logo_width) / 2;
        let right_pad = total_width.saturating_sub(logo_width + left_pad);

        let mut line_spans = vec![Span::raw(" ".repeat(left_pad))];
        line_spans.extend(spans);
        line_spans.push(Span::raw(" ".repeat(right_pad)));

        frame.render_widget(Line::from(line_spans), area);
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
            TimelinePosition::FullDiff => "all changes (base → head) ".to_string(),
            TimelinePosition::Wip => "uncommitted changes ".to_string(),
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
        format!("Files ({})", self.files.len())
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
