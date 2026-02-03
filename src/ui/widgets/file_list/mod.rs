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

use super::Action;

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
}

/// File list widget state
#[derive(Debug, Default)]
pub struct FileListState {
    pub entries: Vec<TreeEntry>,
    pub cursor: usize,
    pub offset: usize,
    pub collapsed: HashSet<String>,
    pub files: Vec<StatusEntry>,
    pub has_comments: HashMap<String, bool>,
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
        if self.cursor >= self.entries.len() && !self.entries.is_empty() {
            self.cursor = self.entries.len() - 1;
        }
    }

    pub fn selected(&self) -> Option<&TreeEntry> {
        self.entries.get(self.cursor)
    }

    pub fn move_down(&mut self) {
        if self.cursor < self.entries.len().saturating_sub(1) {
            self.cursor += 1;
        }
    }

    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_down_n(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.entries.len().saturating_sub(1));
    }

    pub fn move_up_n(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
    }

    pub fn go_top(&mut self) {
        self.cursor = 0;
        self.offset = 0;
    }

    pub fn go_bottom(&mut self) {
        self.cursor = self.entries.len().saturating_sub(1);
    }

    pub fn collapse(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor) {
            if entry.is_dir && !self.collapsed.contains(&entry.path) {
                self.collapsed.insert(entry.path.clone());
                self.rebuild_tree();
            }
        }
    }

    pub fn expand(&mut self) {
        if let Some(entry) = self.entries.get(self.cursor) {
            if entry.is_dir && self.collapsed.contains(&entry.path) {
                self.collapsed.remove(&entry.path);
                self.rebuild_tree();
            }
        }
    }

    pub fn ensure_visible(&mut self, height: usize) {
        let visible_height = height.saturating_sub(3);
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + visible_height {
            self.offset = self.cursor.saturating_sub(visible_height) + 1;
        }
    }

    /// Handle key input, return action for App to dispatch
    pub fn handle_key(&mut self, key: &KeyEvent) -> Action {
        if KeyInput::is_down(key) {
            self.move_down();
            Action::None
        } else if KeyInput::is_up(key) {
            self.move_up();
            Action::None
        } else if KeyInput::is_fast_down(key) {
            self.move_down_n(5);
            Action::None
        } else if KeyInput::is_fast_up(key) {
            self.move_up_n(5);
            Action::None
        } else if KeyInput::is_top(key) {
            self.go_top();
            Action::None
        } else if KeyInput::is_bottom(key) {
            self.go_bottom();
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
        insert_into_tree(&mut root_children, &parts, 0, file.status);
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
    });

    if collapsed.contains("") {
        return entries;
    }

    // Flatten tree with depth-first traversal
    flatten_tree(&root_children, &mut entries, 1, collapsed, has_comments, files);

    entries
}

fn insert_into_tree(nodes: &mut Vec<TreeNode>, parts: &[&str], idx: usize, status: FileStatus) {
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
            insert_into_tree(&mut nodes[i].children, parts, idx + 1, status);
        }
    } else {
        let mut node = TreeNode {
            name: name.to_string(),
            path,
            is_dir: !is_last,
            status: if is_last { status } else { FileStatus::Unchanged },
            children: vec![],
        };

        if !is_last {
            insert_into_tree(&mut node.children, parts, idx + 1, status);
        }

        nodes.push(node);
    }
}

fn sort_tree(nodes: &mut Vec<TreeNode>) {
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
        let border_style = if self.focused {
            self.colors.style_border_focused()
        } else {
            self.colors.style_border()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(&self.title, self.colors.style_header()));

        let inner = block.inner(area);
        block.render(area, buf);

        state.ensure_visible(inner.height as usize);

        let visible_entries: Vec<_> = state
            .entries
            .iter()
            .enumerate()
            .skip(state.offset)
            .take(inner.height as usize)
            .collect();

        for (i, (idx, entry)) in visible_entries.into_iter().enumerate() {
            let y = inner.y + i as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let is_selected = idx == state.cursor;
            let line = render_entry(entry, is_selected, self.colors);

            buf.set_line(inner.x, y, &line, inner.width);
        }
    }
}

fn render_entry(entry: &TreeEntry, selected: bool, colors: &Colors) -> Line<'static> {
    let mut spans = vec![];

    // Cursor
    let cursor = if selected { ">" } else { " " };
    spans.push(Span::raw(cursor.to_string()));

    // Indent
    let indent = "  ".repeat(entry.depth);
    spans.push(Span::raw(indent));

    // Directory prefix
    if entry.is_dir {
        let prefix = if entry.collapsed { "▶ " } else { "▼ " };
        spans.push(Span::styled(prefix.to_string(), colors.style_muted()));
    } else {
        spans.push(Span::raw("  ".to_string()));
    }

    // Name
    let name_style = if selected {
        colors.style_selected()
    } else if entry.is_dir {
        colors.style_muted()
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
