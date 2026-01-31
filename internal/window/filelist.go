package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/keys"
	"github.com/kmacinski/blocks/internal/ui"
)

// FileList displays a list of changed files
type FileList struct {
	Base
	files    []git.FileStatus
	cursor   int
	offset   int // for scrolling
	height   int
	width    int
	onSelect func(index int, path string) tea.Cmd
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
	if f.cursor >= len(files) {
		f.cursor = max(0, len(files)-1)
	}
}

// SetOnSelect sets the callback for when a file is selected
func (f *FileList) SetOnSelect(fn func(index int, path string) tea.Cmd) {
	f.onSelect = fn
}

// SelectedIndex returns the current cursor position
func (f *FileList) SelectedIndex() int {
	return f.cursor
}

// SetSelectedIndex sets the cursor position
func (f *FileList) SetSelectedIndex(index int) {
	if index >= 0 && index < len(f.files) {
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
			if f.cursor < len(f.files)-1 {
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
			f.cursor = max(0, len(f.files)-1)
			f.ensureVisible()
			return f, f.selectCurrent()
		}
	}

	return f, nil
}

func (f *FileList) selectCurrent() tea.Cmd {
	if f.onSelect != nil && f.cursor >= 0 && f.cursor < len(f.files) {
		return f.onSelect(f.cursor, f.files[f.cursor].Path)
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

	if len(f.files) == 0 {
		emptyMsg := f.styles.Muted.Render("No changes")
		lines = append(lines, emptyMsg)
	} else {
		// Render visible files
		for i := f.offset; i < len(f.files) && i < f.offset+contentHeight; i++ {
			file := f.files[i]
			line := f.renderFileLine(file, i == f.cursor, contentWidth)
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

func (f *FileList) renderFileLine(file git.FileStatus, selected bool, maxWidth int) string {
	// Status indicator
	var statusStyle lipgloss.Style
	switch file.Status {
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
	status := statusStyle.Render(file.Status.String())

	// Path
	path := file.Path
	availableWidth := maxWidth - 4 // status + spaces
	if len(path) > availableWidth {
		path = "..." + path[len(path)-availableWidth+3:]
	}

	var pathStyle lipgloss.Style
	if selected {
		pathStyle = f.styles.ListItemSelected
	} else {
		pathStyle = f.styles.ListItem
	}

	// Selection indicator
	cursor := " "
	if selected {
		cursor = ">"
	}

	return fmt.Sprintf("%s %s %s", cursor, pathStyle.Render(path), status)
}
