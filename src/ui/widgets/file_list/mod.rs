use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::config::Colors;
use crate::event::KeyInput;
use crate::git::{FileStatus, StatusEntry};

use super::{Action, ScrollState};

/// Tree node for file display
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub display: String,
    pub path: String,
    pub is_dir: bool,
    pub is_root: bool,
    pub depth: usize,
    pub status: FileStatus,
    pub uncommitted: bool,
    pub collapsed: bool,
    pub children: Vec<String>,
    pub has_comments: bool,
    pub ignored: bool,
}

/// File list widget state
#[derive(Debug, Default)]
pub struct FileListState {
    pub entries: Vec<TreeEntry>,
    pub scroll: ScrollState,
    pub collapsed: HashSet<String>,
    pub files: Vec<StatusEntry>,
    pub has_comments: HashMap<String, bool>,
    // Persisted across timeline switches
    selected_path: Option<String>,
    browse_collapsed: HashSet<String>,
    diff_collapsed: HashSet<String>,
    browse_mode_initialized: bool,
}

impl FileListState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_files(&mut self, files: Vec<StatusEntry>) {
        self.files = files;
        self.rebuild_tree();
    }

    pub fn set_comments(&mut self, comments: HashMap<String, bool>) {
        self.has_comments = comments;
        self.rebuild_tree();
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn rebuild_tree(&mut self) {
        self.entries = build_tree(&self.files, &self.collapsed, &self.has_comments);
        self.scroll.set_len(self.entries.len());
    }

    pub fn selected(&self) -> Option<&TreeEntry> {
        self.entries.get(self.scroll.cursor)
    }

    pub fn collapse(&mut self) {
        if let Some(entry) = self.entries.get(self.scroll.cursor) {
            if entry.is_dir && !self.collapsed.contains(&entry.path) {
                self.collapsed.insert(entry.path.clone());
                self.rebuild_tree();
            }
        }
    }

    pub fn expand(&mut self) {
        if let Some(entry) = self.entries.get(self.scroll.cursor) {
            if entry.is_dir && self.collapsed.contains(&entry.path) {
                self.collapsed.remove(&entry.path);
                self.rebuild_tree();
            }
        }
    }

    /// Save the currently selected path for later restoration
    pub fn save_selected_path(&mut self) {
        if let Some(entry) = self.selected() {
            self.selected_path = Some(entry.path.clone());
        }
    }

    /// Save collapsed state for the mode being left
    pub fn save_mode_state(&mut self, leaving_browse: bool) {
        if leaving_browse {
            self.browse_collapsed = std::mem::take(&mut self.collapsed);
        } else {
            self.diff_collapsed = std::mem::take(&mut self.collapsed);
        }
    }

    /// Restore collapsed state for the mode being entered
    pub fn restore_mode_state(&mut self, entering_browse: bool) {
        self.collapsed = if entering_browse {
            self.browse_collapsed.clone()
        } else {
            self.diff_collapsed.clone()
        };
    }

    /// First-time browse mode setup: collapse all dirs, expand path to selected file
    pub fn initialize_browse_mode(&mut self) {
        if self.browse_mode_initialized {
            return;
        }

        // Collect all directory paths
        let mut all_dirs = HashSet::new();
        for file in &self.files {
            let path = std::path::Path::new(&file.path);
            let mut current = std::path::PathBuf::new();
            for component in path.components() {
                current.push(component);
                if current.to_string_lossy() != file.path {
                    all_dirs.insert(current.to_string_lossy().to_string());
                }
            }
        }

        // Start with all collapsed
        self.collapsed = all_dirs;

        // Expand path to selected file
        if let Some(ref path) = self.selected_path.clone() {
            let file_path = std::path::Path::new(path);
            let mut current = std::path::PathBuf::new();
            for component in file_path.components() {
                current.push(component);
                let dir_str = current.to_string_lossy().to_string();
                if dir_str != *path {
                    self.collapsed.remove(&dir_str);
                }
            }
        }

        self.rebuild_tree();
        self.browse_mode_initialized = true;
    }

    /// Select a path in the file list, or the closest visible parent
    pub fn select_path_or_parent(&mut self, path: &str) {
        // Try exact match first
        if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
            self.scroll.cursor = idx;
            return;
        }

        // Walk up path to find closest visible parent
        let file_path = std::path::Path::new(path);
        let mut components: Vec<_> = file_path.components().collect();
        while !components.is_empty() {
            components.pop();
            if components.is_empty() {
                break;
            }
            let parent: std::path::PathBuf = components.iter().collect();
            let parent_str = parent.to_string_lossy();
            if let Some(idx) = self.entries.iter().position(|e| e.path == parent_str) {
                self.scroll.cursor = idx;
                return;
            }
        }
    }

    /// Restore selection to the saved path (or closest parent)
    pub fn restore_selection(&mut self) {
        if let Some(ref path) = self.selected_path.clone() {
            self.select_path_or_parent(path);
        }
    }

    /// Handle key input, return action for App to dispatch
    pub fn handle_key(&mut self, key: &KeyEvent) -> Action {
        if KeyInput::is_down(key) {
            self.scroll.move_down();
            Action::None
        } else if KeyInput::is_up(key) {
            self.scroll.move_up();
            Action::None
        } else if KeyInput::is_fast_down(key) {
            self.scroll.move_down_n(5);
            Action::None
        } else if KeyInput::is_fast_up(key) {
            self.scroll.move_up_n(5);
            Action::None
        } else if KeyInput::is_top(key) {
            self.scroll.go_top();
            Action::None
        } else if KeyInput::is_bottom(key) {
            self.scroll.go_bottom();
            Action::None
        } else if KeyInput::is_left(key) {
            self.collapse();
            Action::None
        } else if KeyInput::is_right(key) {
            self.expand();
            Action::None
        } else if KeyInput::is_enter(key) {
            // Enter on file -> select it, enter on dir -> expand/collapse
            if let Some(entry) = self.selected() {
                if entry.is_dir {
                    if self.collapsed.contains(&entry.path) {
                        self.expand();
                    } else {
                        self.collapse();
                    }
                    Action::None
                } else {
                    Action::FileSelected(PathBuf::from(&entry.path))
                }
            } else {
                Action::None
            }
        } else {
            Action::Ignored
        }
    }
}

/// Internal tree node for building
struct TreeNode {
    name: String,
    path: String,
    is_dir: bool,
    status: FileStatus,
    children: Vec<TreeNode>,
}

/// Build tree from flat file list
fn build_tree(
    files: &[StatusEntry],
    collapsed: &HashSet<String>,
    has_comments: &HashMap<String, bool>,
) -> Vec<TreeEntry> {
    if files.is_empty() {
        return vec![];
    }

    // Build internal tree structure
    let mut root_children: Vec<TreeNode> = vec![];

    for file in files {
        let parts: Vec<&str> = file.path.split('/').collect();
        insert_into_tree(&mut root_children, &parts, 0, file.status, file.entry_type.is_dir());
    }

    // Sort tree recursively (dirs first at each level, then alphabetically)
    sort_tree(&mut root_children);

    // Flatten to entries
    let mut entries = vec![];

    // Add root entry
    let all_paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    entries.push(TreeEntry {
        display: "./".to_string(),
        path: String::new(),
        is_dir: true,
        is_root: true,
        depth: 0,
        status: FileStatus::Unchanged,
        uncommitted: false,
        collapsed: collapsed.contains(""),
        children: all_paths,
        has_comments: false,
        ignored: false,
    });

    if collapsed.contains("") {
        return entries;
    }

    // Flatten tree with depth-first traversal
    flatten_tree(&root_children, &mut entries, 1, collapsed, has_comments, files);

    entries
}

fn insert_into_tree(nodes: &mut Vec<TreeNode>, parts: &[&str], idx: usize, status: FileStatus, entry_is_dir: bool) {
    if idx >= parts.len() {
        return;
    }

    let name = parts[idx];
    let is_last = idx == parts.len() - 1;
    let path = parts[..=idx].join("/");

    // Find or create node
    let node_idx = nodes.iter().position(|n| n.name == name);

    if let Some(i) = node_idx {
        if !is_last {
            insert_into_tree(&mut nodes[i].children, parts, idx + 1, status, entry_is_dir);
        }
    } else {
        // For the last element, use entry_is_dir to determine if it's a directory
        let node_is_dir = if is_last { entry_is_dir } else { true };
        let mut node = TreeNode {
            name: name.to_string(),
            path,
            is_dir: node_is_dir,
            status: if is_last { status } else { FileStatus::Unchanged },
            children: vec![],
        };

        if !is_last {
            insert_into_tree(&mut node.children, parts, idx + 1, status, entry_is_dir);
        }

        nodes.push(node);
    }
}

fn sort_tree(nodes: &mut [TreeNode]) {
    nodes.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    for node in nodes.iter_mut() {
        sort_tree(&mut node.children);
    }
}

fn flatten_tree(
    nodes: &[TreeNode],
    entries: &mut Vec<TreeEntry>,
    depth: usize,
    collapsed: &HashSet<String>,
    has_comments: &HashMap<String, bool>,
    files: &[StatusEntry],
) {
    for node in nodes {
        let is_collapsed = collapsed.contains(&node.path);

        // Get children paths for directories
        let children: Vec<String> = if node.is_dir {
            files
                .iter()
                .filter(|f| f.path.starts_with(&format!("{}/", node.path)))
                .map(|f| f.path.clone())
                .collect()
        } else {
            vec![]
        };

        // Check if this node or any children have comments
        let node_has_comments = if node.is_dir {
            // For directories, check if any child file has comments
            children.iter().any(|child_path| {
                has_comments.get(child_path).copied().unwrap_or(false)
            })
        } else {
            has_comments.get(&node.path).copied().unwrap_or(false)
        };

        // Check uncommitted status
        let uncommitted = if node.is_dir {
            // For directories, check if any child is uncommitted
            children.iter().any(|child_path| {
                files.iter().any(|f| &f.path == child_path && f.uncommitted)
            })
        } else {
            files.iter().any(|f| f.path == node.path && f.uncommitted)
        };

        // Check ignored status
        let ignored = if node.is_dir {
            // Check if directory itself is ignored, or all children are ignored
            files.iter().any(|f| f.path == node.path && f.entry_type.is_dir() && f.entry_type.is_ignored()) ||
            (!children.is_empty() && children.iter().all(|child_path| {
                files.iter().any(|f| &f.path == child_path && f.entry_type.is_ignored())
            }))
        } else {
            files.iter().any(|f| f.path == node.path && f.entry_type.is_ignored())
        };

        entries.push(TreeEntry {
            display: node.name.clone(),
            path: node.path.clone(),
            is_dir: node.is_dir,
            is_root: false,
            depth,
            status: node.status,
            uncommitted,
            collapsed: is_collapsed,
            children,
            has_comments: node_has_comments,
            ignored,
        });

        // Recurse into children if not collapsed
        if node.is_dir && !is_collapsed {
            flatten_tree(&node.children, entries, depth + 1, collapsed, has_comments, files);
        }
    }
}

/// File list widget
pub struct FileList<'a> {
    colors: &'a Colors,
    focused: bool,
    title: String,
}

impl<'a> FileList<'a> {
    pub fn new(colors: &'a Colors) -> Self {
        Self {
            colors,
            focused: false,
            title: "Files".to_string(),
        }
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl<'a> StatefulWidget for FileList<'a> {
    type State = FileListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_style = self.colors.border_style(self.focused);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(&self.title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        state.scroll.ensure_visible(inner.height as usize);

        let visible_entries: Vec<_> = state
            .entries
            .iter()
            .enumerate()
            .skip(state.scroll.offset)
            .take(inner.height as usize)
            .collect();

        for (i, (idx, entry)) in visible_entries.into_iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = idx == state.scroll.cursor;
            let line = render_entry(entry, is_selected, self.colors);

            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::EntryType;

    fn make_entry(path: &str, status: FileStatus) -> StatusEntry {
        StatusEntry {
            path: path.to_string(),
            status,
            uncommitted: false,
            entry_type: EntryType::Tracked,
        }
    }

    // --- Tree building ---

    #[test]
    fn build_tree_empty() {
        let entries = build_tree(&[], &HashSet::new(), &HashMap::new());
        assert!(entries.is_empty());
    }

    #[test]
    fn build_tree_single_file() {
        let files = vec![make_entry("README.md", FileStatus::Added)];
        let entries = build_tree(&files, &HashSet::new(), &HashMap::new());
        // Root + file
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_root);
        assert_eq!(entries[1].display, "README.md");
        assert_eq!(entries[1].status, FileStatus::Added);
    }

    #[test]
    fn build_tree_nested_structure() {
        let files = vec![
            make_entry("src/main.rs", FileStatus::Modified),
            make_entry("src/lib.rs", FileStatus::Added),
            make_entry("README.md", FileStatus::Unchanged),
        ];
        let entries = build_tree(&files, &HashSet::new(), &HashMap::new());
        // Root, src/ dir, lib.rs, main.rs, README.md
        assert_eq!(entries.len(), 5);
        assert!(entries[0].is_root); // ./
        assert!(entries[1].is_dir);  // src/
        assert_eq!(entries[1].display, "src");
        // Files under src sorted alphabetically
        assert_eq!(entries[2].display, "lib.rs");
        assert_eq!(entries[3].display, "main.rs");
        // Top-level file after directories
        assert_eq!(entries[4].display, "README.md");
    }

    #[test]
    fn build_tree_dirs_sorted_before_files() {
        let files = vec![
            make_entry("zebra.txt", FileStatus::Added),
            make_entry("alpha/file.rs", FileStatus::Modified),
        ];
        let entries = build_tree(&files, &HashSet::new(), &HashMap::new());
        // Root, alpha/ dir, file.rs, zebra.txt
        assert!(entries[1].is_dir);
        assert_eq!(entries[1].display, "alpha");
        assert_eq!(entries[3].display, "zebra.txt");
    }

    #[test]
    fn build_tree_collapsed_root_hides_children() {
        let files = vec![make_entry("a.txt", FileStatus::Added)];
        let mut collapsed = HashSet::new();
        collapsed.insert(String::new()); // collapse root
        let entries = build_tree(&files, &collapsed, &HashMap::new());
        assert_eq!(entries.len(), 1); // root only
        assert!(entries[0].collapsed);
    }

    #[test]
    fn build_tree_collapsed_dir() {
        let files = vec![
            make_entry("src/a.rs", FileStatus::Modified),
            make_entry("src/b.rs", FileStatus::Modified),
        ];
        let mut collapsed = HashSet::new();
        collapsed.insert("src".to_string());
        let entries = build_tree(&files, &collapsed, &HashMap::new());
        // Root + collapsed src dir (children hidden)
        assert_eq!(entries.len(), 2);
        assert!(entries[1].collapsed);
    }

    // --- Navigation ---

    #[test]
    fn navigation_bounds() {
        let mut state = FileListState::new();
        state.set_files(vec![
            make_entry("a.txt", FileStatus::Added),
            make_entry("b.txt", FileStatus::Added),
        ]);
        // Root + 2 files = 3 entries
        assert_eq!(state.scroll.cursor, 0);
        state.scroll.move_up(); // already at top
        assert_eq!(state.scroll.cursor, 0);
        state.scroll.go_bottom();
        assert_eq!(state.scroll.cursor, 2);
        state.scroll.move_down(); // already at bottom
        assert_eq!(state.scroll.cursor, 2);
    }

    #[test]
    fn navigation_fast_move() {
        let mut state = FileListState::new();
        state.set_files(vec![
            make_entry("a.txt", FileStatus::Added),
            make_entry("b.txt", FileStatus::Added),
            make_entry("c.txt", FileStatus::Added),
        ]);
        // 4 entries (root + 3 files)
        state.scroll.move_down_n(5); // clamped to last
        assert_eq!(state.scroll.cursor, 3);
        state.scroll.move_up_n(5); // clamped to first
        assert_eq!(state.scroll.cursor, 0);
    }

    #[test]
    fn collapse_expand() {
        let mut state = FileListState::new();
        state.set_files(vec![
            make_entry("src/a.rs", FileStatus::Modified),
            make_entry("src/b.rs", FileStatus::Modified),
        ]);
        // Root, src/, a.rs, b.rs = 4
        assert_eq!(state.entries.len(), 4);

        state.scroll.cursor = 1; // select src/
        state.collapse();
        assert_eq!(state.entries.len(), 2); // root + collapsed src

        state.scroll.cursor = 1;
        state.expand();
        assert_eq!(state.entries.len(), 4); // expanded again
    }
}

fn render_entry(entry: &TreeEntry, selected: bool, colors: &Colors) -> Line<'static> {
    let mut spans = vec![];

    // Cursor
    let cursor = if selected { ">" } else { " " };
    spans.push(Span::raw(cursor.to_string()));

    // Indent (1 space per level)
    let indent = " ".repeat(entry.depth);
    spans.push(Span::raw(indent));

    // Directory prefix
    if entry.is_dir {
        let prefix = if entry.collapsed { "▶ " } else { "▼ " };
        spans.push(Span::styled(prefix.to_string(), colors.style_muted()));
    } else {
        spans.push(Span::raw("  ".to_string()));
    }

    // Name (directories in header color, files in text color, ignored dimmed)
    let name_style = if selected {
        colors.style_selected()
    } else if entry.ignored {
        colors.style_muted() // Dim ignored files/folders
    } else if entry.is_dir {
        colors.style_header() // Directories in distinct color
    } else {
        Style::reset().fg(colors.text)
    };
    spans.push(Span::styled(entry.display.clone(), name_style));

    // Status
    if !entry.is_dir && entry.status != FileStatus::Unchanged {
        let status_style = match entry.status {
            FileStatus::Modified => colors.style_modified(),
            FileStatus::Added => colors.style_added(),
            FileStatus::Deleted => colors.style_removed(),
            FileStatus::Renamed => Style::reset().fg(colors.renamed),
            FileStatus::Unchanged => colors.style_muted(),
        };
        spans.push(Span::raw(" ".to_string()));
        spans.push(Span::styled(entry.status.to_string(), status_style));
    }

    // Uncommitted indicator
    if entry.uncommitted {
        spans.push(Span::raw(" ".to_string()));
        spans.push(Span::styled("●".to_string(), colors.style_modified()));
    }

    // Comment indicator
    if entry.has_comments {
        spans.push(Span::raw(" ".to_string()));
        spans.push(Span::styled("C".to_string(), colors.style_header()));
    }

    Line::from(spans)
}
