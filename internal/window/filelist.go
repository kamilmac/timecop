package window

import (
	"fmt"
	"path/filepath"
	"sort"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/github"
	"github.com/kmacinski/blocks/internal/keys"
	"github.com/kmacinski/blocks/internal/ui"
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
	display  string
	path     string // full path (for files) or dir path (for folders)
	isDir    bool
	isRoot   bool // special root entry for PR summary
	depth    int
	status   git.Status
	children []string // paths of child files (for directories)
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
}

// NewFileList creates a new file list window
func NewFileList(styles ui.Styles) *FileList {
	return &FileList{
		Base: NewBase("filelist", styles),
	}
}

// SetFiles updates the file list
func (f *FileList) SetFiles(files []git.FileStatus) {
	f.files = files
	f.flatEntries = f.buildTree(files)
	if f.cursor >= len(f.flatEntries) {
		f.cursor = max(0, len(f.flatEntries)-1)
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

	// Add root entry (represents whole repo / PR summary)
	entries = append(entries, flatEntry{
		display:  "./",
		path:     "",
		isDir:    true,
		isRoot:   true,
		depth:    0,
		children: allPaths,
	})

	// Build directory children map
	dirChildren := buildDirChildrenMap(files)

	flattenTreeWithChildren(root, &entries, 0, dirChildren)

	return entries
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

func flattenTree(node *treeNode, entries *[]flatEntry, depth int) {
	for _, child := range node.children {
		*entries = append(*entries, flatEntry{
			display: child.name,
			path:    child.path,
			isDir:   child.isDir,
			depth:   depth,
			status:  child.status,
		})
		if child.isDir {
			flattenTree(child, entries, depth+1)
		}
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

func flattenTreeWithChildren(node *treeNode, entries *[]flatEntry, depth int, dirChildren map[string][]string) {
	for _, child := range node.children {
		entry := flatEntry{
			display: child.name,
			path:    child.path,
			isDir:   child.isDir,
			depth:   depth,
			status:  child.status,
		}
		if child.isDir {
			entry.children = dirChildren[child.path]
		}
		*entries = append(*entries, entry)
		if child.isDir {
			flattenTreeWithChildren(child, entries, depth+1, dirChildren)
		}
	}
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
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			f.cursor = 0
			f.offset = 0
			return f, f.selectCurrent()
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			f.cursor = max(0, len(f.flatEntries)-1)
			f.ensureVisible()
			return f, f.selectCurrent()
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
	title := "Files"
	if len(f.files) > 0 {
		title = fmt.Sprintf("Files (%d)", len(f.files))
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
		prefix = "▼ "
	} else {
		prefix = "  "
	}

	// Name
	name := entry.display

	// Status indicator (only for files with changes)
	var statusStr string
	if !entry.isDir && entry.status != git.StatusUnchanged {
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

	// Comment indicator
	var commentStr string
	if !entry.isDir && f.pr != nil && len(f.pr.FileComments[entry.path]) > 0 {
		commentStr = " " + f.styles.DiffHeader.Render("C")
	}

	// Calculate available width for name
	indentLen := len(indent)
	prefixLen := 2 // "▼ " or "  "
	statusLen := 0
	if !entry.isDir && entry.status != git.StatusUnchanged {
		statusLen = 3 // " M" or similar
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
