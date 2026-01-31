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
	display string
	path    string // empty for directories
	isDir   bool
	depth   int
	status  git.Status
}

// FileList displays a list of changed files
type FileList struct {
	Base
	files       []git.FileStatus
	flatEntries []flatEntry // flattened tree for display
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
	// Ensure cursor is on a file, not directory
	f.skipToFile(1)
}

func (f *FileList) buildTree(files []git.FileStatus) []flatEntry {
	if len(files) == 0 {
		return nil
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
				child = &treeNode{
					name:  part,
					isDir: !isLast,
					depth: i,
				}
				if isLast {
					child.path = file.Path
					child.status = file.Status
				}
				current.children = append(current.children, child)
			}
			current = child
		}
	}

	// Sort children at each level (dirs first, then alphabetically)
	sortTree(root)

	// Flatten tree for display
	var entries []flatEntry
	flattenTree(root, &entries, 0)

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

func (f *FileList) skipToFile(direction int) {
	// Skip directory entries when navigating
	for f.cursor >= 0 && f.cursor < len(f.flatEntries) && f.flatEntries[f.cursor].isDir {
		f.cursor += direction
	}
	// Clamp
	if f.cursor < 0 {
		f.cursor = 0
		// Find first file
		for f.cursor < len(f.flatEntries) && f.flatEntries[f.cursor].isDir {
			f.cursor++
		}
	}
	if f.cursor >= len(f.flatEntries) {
		f.cursor = len(f.flatEntries) - 1
		// Find last file
		for f.cursor >= 0 && f.flatEntries[f.cursor].isDir {
			f.cursor--
		}
	}
}

// SetOnSelect sets the callback for when a file is selected
func (f *FileList) SetOnSelect(fn func(index int, path string) tea.Cmd) {
	f.onSelect = fn
}

// SelectedIndex returns the index of the selected file in the original files slice
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

// SelectedPath returns the path of the currently selected file
func (f *FileList) SelectedPath() string {
	if f.cursor < 0 || f.cursor >= len(f.flatEntries) {
		return ""
	}
	return f.flatEntries[f.cursor].path
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
				// Skip directories
				for f.cursor < len(f.flatEntries)-1 && f.flatEntries[f.cursor].isDir {
					f.cursor++
				}
				f.ensureVisible()
				return f, f.selectCurrent()
			}
		case key.Matches(msg, keys.DefaultKeyMap.Up):
			if f.cursor > 0 {
				f.cursor--
				// Skip directories
				for f.cursor > 0 && f.flatEntries[f.cursor].isDir {
					f.cursor--
				}
				f.ensureVisible()
				return f, f.selectCurrent()
			}
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			f.cursor = 0
			f.skipToFile(1)
			f.offset = 0
			return f, f.selectCurrent()
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			f.cursor = max(0, len(f.flatEntries)-1)
			f.skipToFile(-1)
			f.ensureVisible()
			return f, f.selectCurrent()
		}
	}

	return f, nil
}

func (f *FileList) selectCurrent() tea.Cmd {
	if f.onSelect != nil && f.cursor >= 0 && f.cursor < len(f.flatEntries) {
		entry := f.flatEntries[f.cursor]
		if !entry.isDir {
			return f.onSelect(f.SelectedIndex(), entry.path)
		}
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

	// Pad remaining lines
	for len(lines) < height-2 {
		lines = append(lines, "")
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

	// Calculate available width for name
	indentLen := len(indent)
	prefixLen := 2 // "▼ " or "  "
	statusLen := 0
	if !entry.isDir && entry.status != git.StatusUnchanged {
		statusLen = 3 // " M" or similar
	}
	cursorLen := 2 // "> " or "  "

	availableWidth := maxWidth - indentLen - prefixLen - statusLen - cursorLen
	if availableWidth < 1 {
		availableWidth = 1
	}
	if len(name) > availableWidth {
		name = name[:availableWidth-3] + "..."
	}

	// Style based on selection and type
	var nameStyle lipgloss.Style
	if entry.isDir {
		nameStyle = f.styles.Muted
	} else if selected {
		nameStyle = f.styles.ListItemSelected
	} else {
		nameStyle = f.styles.ListItem
	}

	// Selection indicator (only for files)
	cursor := " "
	if selected && !entry.isDir {
		cursor = ">"
	}

	return fmt.Sprintf("%s%s%s%s%s", cursor, indent, prefix, nameStyle.Render(name), statusStr)
}
