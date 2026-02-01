package window

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/config"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/github"
	"github.com/kmacinski/blocks/internal/keys"
)

// treeNode represents a node in the file tree
type treeNode struct {
	name     string
	path     string         // full path for files
	isDir    bool
	status   git.Status
	children []*treeNode
	depth    int
}

// flatEntry is a flattened tree entry for display/navigation
type flatEntry struct {
	display     string
	path        string // full path (for files) or dir path (for folders)
	isDir       bool
	isRoot      bool // special root entry for PR summary
	depth       int
	status      git.Status
	children    []string     // paths of child files (for directories)
	collapsed   bool         // true if folder is collapsed
	childStats  []git.Status // aggregated statuses from children (for collapsed folders)
	hasComments bool         // true if any child has comments (for collapsed folders)
}

// FileList displays a list of changed files
type FileList struct {
	Base
	files       []git.FileStatus
	flatEntries []flatEntry // flattened tree for display
	pr          *github.PRInfo
	cursor      int
	offset      int // for scrolling
	height      int
	width       int
	onSelect    func(index int, path string) tea.Cmd
	collapsed   map[string]bool  // tracks collapsed folders by path
	viewMode    git.FileViewMode // current view mode
}

// NewFileList creates a new file list window
func NewFileList(styles config.Styles) *FileList {
	return &FileList{
		Base:      NewBase("filelist", styles),
		collapsed: make(map[string]bool),
	}
}

// SetFiles updates the file list
func (f *FileList) SetFiles(files []git.FileStatus) {
	f.files = files

	// Auto-collapse leaf folders when in browse mode
	if f.viewMode == git.FileViewAll {
		f.collapsed = make(map[string]bool) // reset first
		f.collapseLeafFolders()
	}

	f.flatEntries = f.buildTree(files)
	if f.cursor >= len(f.flatEntries) {
		f.cursor = max(0, len(f.flatEntries)-1)
	}
}

// SetViewMode updates the view mode and adjusts collapsed state
func (f *FileList) SetViewMode(mode git.FileViewMode) {
	prevMode := f.viewMode
	f.viewMode = mode

	// When leaving "all files" mode, clear collapsed state
	if mode != git.FileViewAll && prevMode == git.FileViewAll {
		f.collapsed = make(map[string]bool)
	}

	// Note: auto-collapse happens in SetFiles() when new files are loaded
	// This ensures we have the correct file list before collapsing
}

// collapseLeafFolders collapses all folders that contain only files (no subdirectories)
// Only collapses folders at or beyond TreeAutoCollapseDepth
func (f *FileList) collapseLeafFolders() {
	// Build a set of all directory paths with their depths
	dirPaths := make(map[string]int) // path -> depth
	for _, file := range f.files {
		parts := strings.Split(file.Path, string(filepath.Separator))
		for i := 1; i < len(parts); i++ {
			dirPath := strings.Join(parts[:i], string(filepath.Separator))
			dirPaths[dirPath] = i - 1 // depth is 0-indexed (0 = first level like "internal")
		}
	}

	// Find leaf folders (folders that don't have any subdirectories)
	leafFolders := make(map[string]int) // path -> depth
	for dir, depth := range dirPaths {
		isLeaf := true
		for otherDir := range dirPaths {
			if otherDir != dir && strings.HasPrefix(otherDir, dir+string(filepath.Separator)) {
				isLeaf = false
				break
			}
		}
		if isLeaf {
			leafFolders[dir] = depth
		}
	}

	// Collapse leaf folders at or beyond the configured depth
	for dir, depth := range leafFolders {
		if depth >= config.TreeAutoCollapseDepth {
			f.collapsed[dir] = true
		}
	}
}

func (f *FileList) buildTree(files []git.FileStatus) []flatEntry {
	if len(files) == 0 {
		return nil
	}

	// Collect all file paths for root entry
	allPaths := make([]string, len(files))
	for i, file := range files {
		allPaths[i] = file.Path
	}

	// Build tree structure
	root := &treeNode{isDir: true}

	for _, file := range files {
		parts := strings.Split(file.Path, string(filepath.Separator))
		current := root

		for i, part := range parts {
			isLast := i == len(parts)-1

			// Find or create child
			var child *treeNode
			for _, c := range current.children {
				if c.name == part {
					child = c
					break
				}
			}

			if child == nil {
				// Build path for this node
				nodePath := strings.Join(parts[:i+1], string(filepath.Separator))
				child = &treeNode{
					name:  part,
					path:  nodePath,
					isDir: !isLast,
					depth: i,
				}
				if isLast {
					child.status = file.Status
				}
				current.children = append(current.children, child)
			}
			current = child
		}
	}

	// Sort children at each level (dirs first, then alphabetically)
	sortTree(root)

	// Flatten tree for display - start with root entry
	var entries []flatEntry

	// Build directory children map and status map
	dirChildren := buildDirChildrenMap(files)
	fileStatusMap := make(map[string]git.Status)
	for _, file := range files {
		fileStatusMap[file.Path] = file.Status
	}

	// Add root entry (represents whole repo / PR summary)
	rootCollapsed := f.collapsed[""]
	rootEntry := flatEntry{
		display:   "./",
		path:      "",
		isDir:     true,
		isRoot:    true,
		depth:     0,
		children:  allPaths,
		collapsed: rootCollapsed,
	}
	if rootCollapsed {
		rootEntry.childStats, rootEntry.hasComments = f.aggregateChildStats(allPaths, fileStatusMap)
	}
	entries = append(entries, rootEntry)

	// Don't show children if root is collapsed
	if !rootCollapsed {
		f.flattenTreeWithCollapse(root, &entries, 0, dirChildren, fileStatusMap)
	}

	return entries
}

// flattenTreeWithCollapse flattens tree respecting collapsed state
func (f *FileList) flattenTreeWithCollapse(node *treeNode, entries *[]flatEntry, depth int, dirChildren map[string][]string, fileStatusMap map[string]git.Status) {
	for _, child := range node.children {
		isCollapsed := f.collapsed[child.path]

		entry := flatEntry{
			display:   child.name,
			path:      child.path,
			isDir:     child.isDir,
			depth:     depth,
			status:    child.status,
			collapsed: isCollapsed,
		}

		if child.isDir {
			entry.children = dirChildren[child.path]
			if isCollapsed {
				entry.childStats, entry.hasComments = f.aggregateChildStats(entry.children, fileStatusMap)
			}
		}

		*entries = append(*entries, entry)

		// Only recurse into non-collapsed directories
		if child.isDir && !isCollapsed {
			f.flattenTreeWithCollapse(child, entries, depth+1, dirChildren, fileStatusMap)
		}
	}
}

// aggregateChildStats returns unique statuses and comment presence for child files
func (f *FileList) aggregateChildStats(children []string, fileStatusMap map[string]git.Status) ([]git.Status, bool) {
	statusSet := make(map[git.Status]bool)
	hasComments := false

	for _, path := range children {
		if status, ok := fileStatusMap[path]; ok && status != git.StatusUnchanged {
			statusSet[status] = true
		}
		// Check for comments
		if f.pr != nil && len(f.pr.FileComments[path]) > 0 {
			hasComments = true
		}
	}

	// Convert to slice in priority order: D, M, A, R, ?
	var stats []git.Status
	priority := []git.Status{git.StatusDeleted, git.StatusModified, git.StatusAdded, git.StatusRenamed, git.StatusUntracked}
	for _, s := range priority {
		if statusSet[s] {
			stats = append(stats, s)
		}
	}

	return stats, hasComments
}

func sortTree(node *treeNode) {
	sort.Slice(node.children, func(i, j int) bool {
		// Directories first
		if node.children[i].isDir != node.children[j].isDir {
			return node.children[i].isDir
		}
		return node.children[i].name < node.children[j].name
	})
	for _, child := range node.children {
		sortTree(child)
	}
}

// buildDirChildrenMap creates a map of directory paths to their child file paths
func buildDirChildrenMap(files []git.FileStatus) map[string][]string {
	result := make(map[string][]string)
	for _, file := range files {
		parts := strings.Split(file.Path, string(filepath.Separator))
		// Add to each parent directory
		for i := 1; i <= len(parts)-1; i++ {
			dirPath := strings.Join(parts[:i], string(filepath.Separator))
			result[dirPath] = append(result[dirPath], file.Path)
		}
	}
	return result
}


// SetOnSelect sets the callback for when a file is selected
func (f *FileList) SetOnSelect(fn func(index int, path string) tea.Cmd) {
	f.onSelect = fn
}

// SetPR sets the PR info for comment indicators
func (f *FileList) SetPR(pr *github.PRInfo) {
	f.pr = pr
}

// SelectedIndex returns the index of the selected file in the original files slice
// Returns -1 for directories
func (f *FileList) SelectedIndex() int {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return -1
	}
	entry := f.flatEntries[f.cursor]
	if entry.isDir {
		return -1
	}
	// Find index in original files
	for i, file := range f.files {
		if file.Path == entry.path {
			return i
		}
	}
	return -1
}

// SelectedPath returns the path of the currently selected item
func (f *FileList) SelectedPath() string {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return ""
	}
	return f.flatEntries[f.cursor].path
}

// SelectedEntry returns the currently selected entry
func (f *FileList) SelectedEntry() *flatEntry {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return nil
	}
	return &f.flatEntries[f.cursor]
}

// IsRootSelected returns true if the root entry is selected
func (f *FileList) IsRootSelected() bool {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return false
	}
	return f.flatEntries[f.cursor].isRoot
}

// IsFolderSelected returns true if a folder (including root) is selected
func (f *FileList) IsFolderSelected() bool {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return false
	}
	return f.flatEntries[f.cursor].isDir
}

// SelectedChildren returns the child file paths if a folder is selected
func (f *FileList) SelectedChildren() []string {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return nil
	}
	return f.flatEntries[f.cursor].children
}

// SetSelectedIndex sets the cursor position
func (f *FileList) SetSelectedIndex(index int) {
	if index >= 0 && index < len(f.flatEntries) {
		f.cursor = index
		f.ensureVisible()
	}
}

// Update handles input
func (f *FileList) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !f.focused {
		return f, nil
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, keys.DefaultKeyMap.Down):
			if f.cursor < len(f.flatEntries)-1 {
				f.cursor++
				f.ensureVisible()
				return f, f.selectCurrent()
			}
		case key.Matches(msg, keys.DefaultKeyMap.Up):
			if f.cursor > 0 {
				f.cursor--
				f.ensureVisible()
				return f, f.selectCurrent()
			}
		case key.Matches(msg, keys.DefaultKeyMap.FastDown):
			f.cursor = min(f.cursor+5, len(f.flatEntries)-1)
			f.ensureVisible()
			return f, f.selectCurrent()
		case key.Matches(msg, keys.DefaultKeyMap.FastUp):
			f.cursor = max(f.cursor-5, 0)
			f.ensureVisible()
			return f, f.selectCurrent()
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			f.cursor = 0
			f.offset = 0
			return f, f.selectCurrent()
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			f.cursor = max(0, len(f.flatEntries)-1)
			f.ensureVisible()
			return f, f.selectCurrent()
		case key.Matches(msg, keys.DefaultKeyMap.Left):
			// Collapse folder (h)
			if f.cursor >= 0 && f.cursor < len(f.flatEntries) {
				entry := f.flatEntries[f.cursor]
				if entry.isDir && !entry.collapsed {
					f.collapsed[entry.path] = true
					f.flatEntries = f.buildTree(f.files)
					f.ensureVisible()
					return f, f.selectCurrent()
				}
			}
		case key.Matches(msg, keys.DefaultKeyMap.Right):
			// Expand folder (l)
			if f.cursor >= 0 && f.cursor < len(f.flatEntries) {
				entry := f.flatEntries[f.cursor]
				if entry.isDir && entry.collapsed {
					delete(f.collapsed, entry.path)
					f.flatEntries = f.buildTree(f.files)
					f.ensureVisible()
					return f, f.selectCurrent()
				}
			}
		}
	}

	return f, nil
}

func (f *FileList) selectCurrent() tea.Cmd {
	if f.onSelect != nil && f.cursor >= 0 && f.cursor < len(f.flatEntries) {
		entry := f.flatEntries[f.cursor]
		return f.onSelect(f.SelectedIndex(), entry.path)
	}
	return nil
}

func (f *FileList) ensureVisible() {
	visibleHeight := f.height - 3 // Account for border and title
	if visibleHeight < 1 {
		visibleHeight = 1
	}

	if f.cursor < f.offset {
		f.offset = f.cursor
	} else if f.cursor >= f.offset+visibleHeight {
		f.offset = f.cursor - visibleHeight + 1
	}
}

// View renders the file list
func (f *FileList) View(width, height int) string {
	f.width = width
	f.height = height

	var style lipgloss.Style
	if f.focused {
		style = f.styles.WindowFocused
	} else {
		style = f.styles.WindowUnfocused
	}

	// Calculate content dimensions
	contentWidth := width - 2  // borders
	contentHeight := height - 2 // borders

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	// Build content
	var lines []string

	// Title
	var title string
	if f.viewMode == git.FileViewAll {
		title = "Browse"
		if len(f.files) > 0 {
			title = fmt.Sprintf("Browse (%d)", len(f.files))
		}
	} else {
		title = "Files"
		if len(f.files) > 0 {
			title = fmt.Sprintf("Files (%d)", len(f.files))
		}
	}
	titleLine := f.styles.WindowTitle.Render(title)
	lines = append(lines, titleLine)
	contentHeight-- // Account for title

	if len(f.flatEntries) == 0 {
		emptyMsg := f.styles.Muted.Render("No changes")
		lines = append(lines, emptyMsg)
	} else {
		// Render visible entries
		for i := f.offset; i < len(f.flatEntries) && i < f.offset+contentHeight; i++ {
			entry := f.flatEntries[i]
			line := f.renderTreeLine(entry, i == f.cursor, contentWidth)
			lines = append(lines, line)
		}
	}

	// Ensure exactly height-2 lines (pad or truncate)
	targetLines := height - 2
	for len(lines) < targetLines {
		lines = append(lines, "")
	}
	if len(lines) > targetLines {
		lines = lines[:targetLines]
	}

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		Height(height - 2).
		Render(content)
}

func (f *FileList) renderTreeLine(entry flatEntry, selected bool, maxWidth int) string {
	// Indentation based on depth
	indent := strings.Repeat("  ", entry.depth)

	// Icon/prefix
	var prefix string
	if entry.isDir {
		if entry.collapsed {
			prefix = "▶ "
		} else {
			prefix = "▼ "
		}
	} else {
		prefix = "  "
	}

	// Name
	name := entry.display

	// Status indicator (hidden in "all files" mode)
	var statusStr string
	if f.viewMode != git.FileViewAll {
		if entry.isDir && entry.collapsed && len(entry.childStats) > 0 {
			// Aggregated status for collapsed folders
			var parts []string
			for _, s := range entry.childStats {
				var statusStyle lipgloss.Style
				switch s {
				case git.StatusModified:
					statusStyle = f.styles.StatusModified
				case git.StatusAdded:
					statusStyle = f.styles.StatusAdded
				case git.StatusDeleted:
					statusStyle = f.styles.StatusDeleted
				case git.StatusUntracked:
					statusStyle = f.styles.StatusUntracked
				case git.StatusRenamed:
					statusStyle = f.styles.StatusRenamed
				}
				parts = append(parts, statusStyle.Render(s.String()))
			}
			statusStr = " " + strings.Join(parts, "")
		} else if !entry.isDir && entry.status != git.StatusUnchanged {
			// Single file status
			var statusStyle lipgloss.Style
			switch entry.status {
			case git.StatusModified:
				statusStyle = f.styles.StatusModified
			case git.StatusAdded:
				statusStyle = f.styles.StatusAdded
			case git.StatusDeleted:
				statusStyle = f.styles.StatusDeleted
			case git.StatusUntracked:
				statusStyle = f.styles.StatusUntracked
			case git.StatusRenamed:
				statusStyle = f.styles.StatusRenamed
			}
			statusStr = " " + statusStyle.Render(entry.status.String())
		}
	}

	// Comment indicator (hidden in "all files" mode)
	var commentStr string
	if f.viewMode != git.FileViewAll {
		if entry.isDir && entry.collapsed && entry.hasComments {
			// Collapsed folder with comments in children
			commentStr = " " + f.styles.DiffHeader.Render("C")
		} else if !entry.isDir && f.pr != nil && len(f.pr.FileComments[entry.path]) > 0 {
			// Single file with comments
			commentStr = " " + f.styles.DiffHeader.Render("C")
		}
	}

	// Calculate available width for name
	indentLen := len(indent)
	prefixLen := 2 // "▼ " or "▶ " or "  "
	statusLen := 0
	if statusStr != "" {
		// Rough estimate: each status is 2 chars (space + letter)
		if entry.isDir && entry.collapsed {
			statusLen = 1 + len(entry.childStats)
		} else if !entry.isDir && entry.status != git.StatusUnchanged {
			statusLen = 2
		}
	}
	commentLen := 0
	if commentStr != "" {
		commentLen = 2 // " C"
	}
	cursorLen := 2 // "> " or "  "

	availableWidth := maxWidth - indentLen - prefixLen - statusLen - commentLen - cursorLen
	if availableWidth < 1 {
		availableWidth = 1
	}
	if len(name) > availableWidth {
		name = name[:availableWidth-3] + "..."
	}

	// Style based on selection and type
	var nameStyle lipgloss.Style
	if selected {
		nameStyle = f.styles.ListItemSelected
	} else if entry.isDir {
		nameStyle = f.styles.Muted
	} else {
		nameStyle = f.styles.ListItem
	}

	// Selection indicator
	cursor := " "
	if selected {
		cursor = ">"
	}

	return fmt.Sprintf("%s%s%s%s%s%s", cursor, indent, prefix, nameStyle.Render(name), statusStr, commentStr)
}
