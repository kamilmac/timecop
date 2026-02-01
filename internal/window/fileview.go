package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/config"
	"github.com/kmacinski/blocks/internal/keys"
)

// FileView displays file content for browsing
type FileView struct {
	Base
	content  string
	lines    []string
	filePath string
	cursor   int // current line (for scrolling)
	width    int
	height   int
}

// NewFileView creates a new file view window
func NewFileView(styles config.Styles) *FileView {
	return &FileView{
		Base: NewBase("fileview", styles),
	}
}

// SetContent sets the file content to display
func (f *FileView) SetContent(content, path string) {
	f.content = content
	f.filePath = path
	f.cursor = 0

	if content == "" {
		f.lines = nil
	} else {
		f.lines = strings.Split(content, "\n")
	}
}

// GetFilePath returns the current file path
func (f *FileView) GetFilePath() string {
	return f.filePath
}

// GetSelectedLine returns the current cursor line number (1-indexed)
func (f *FileView) GetSelectedLine() int {
	if len(f.lines) == 0 {
		return 0
	}
	return f.cursor + 1
}

// Update handles input
func (f *FileView) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !f.focused {
		return f, nil
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		visibleLines := f.height - 4 // borders + title
		if visibleLines < 1 {
			visibleLines = 1
		}

		switch {
		case key.Matches(msg, keys.DefaultKeyMap.Down):
			if f.cursor < len(f.lines)-1 {
				f.cursor++
			}
		case key.Matches(msg, keys.DefaultKeyMap.Up):
			if f.cursor > 0 {
				f.cursor--
			}
		case key.Matches(msg, keys.DefaultKeyMap.FastDown):
			f.cursor = min(f.cursor+5, max(0, len(f.lines)-1))
		case key.Matches(msg, keys.DefaultKeyMap.FastUp):
			f.cursor = max(f.cursor-5, 0)
		case key.Matches(msg, keys.DefaultKeyMap.HalfPgDn):
			f.cursor = min(f.cursor+visibleLines/2, max(0, len(f.lines)-1))
		case key.Matches(msg, keys.DefaultKeyMap.HalfPgUp):
			f.cursor = max(f.cursor-visibleLines/2, 0)
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			f.cursor = 0
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			f.cursor = max(0, len(f.lines)-1)
		}
	}

	return f, nil
}

// View renders the file content
func (f *FileView) View(width, height int) string {
	f.width = width
	f.height = height

	var style lipgloss.Style
	if f.focused {
		style = f.styles.WindowFocused
	} else {
		style = f.styles.WindowUnfocused
	}

	contentWidth := width - 2
	contentHeight := height - 2

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	var lines []string

	// Title with file name and scroll position
	title := f.renderTitle(contentHeight)
	lines = append(lines, title)
	contentHeight--

	if len(f.lines) == 0 {
		if f.filePath == "" {
			lines = append(lines, f.styles.Muted.Render("Select a file to view"))
		} else {
			lines = append(lines, f.styles.Muted.Render("Empty file"))
		}
	} else {
		// Calculate visible range, keeping cursor in view
		startLine := f.cursor - contentHeight/2
		if startLine < 0 {
			startLine = 0
		}
		if startLine > len(f.lines)-contentHeight {
			startLine = max(0, len(f.lines)-contentHeight)
		}

		endLine := startLine + contentHeight
		if endLine > len(f.lines) {
			endLine = len(f.lines)
		}

		// Line number width
		lineNumWidth := len(fmt.Sprintf("%d", len(f.lines)))
		if lineNumWidth < 3 {
			lineNumWidth = 3
		}

		// Render visible lines
		for i := startLine; i < endLine; i++ {
			line := f.renderLine(i, lineNumWidth, contentWidth)
			lines = append(lines, line)
		}
	}

	// Pad to fill height
	for len(lines) < height-2 {
		lines = append(lines, "")
	}
	if len(lines) > height-2 {
		lines = lines[:height-2]
	}

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		Height(height - 2).
		Render(content)
}

func (f *FileView) renderTitle(contentHeight int) string {
	// File name
	name := f.filePath
	if name == "" {
		name = "Preview"
	}

	// Scroll position
	var pos string
	if len(f.lines) <= contentHeight {
		pos = ""
	} else if f.cursor == 0 {
		pos = " [top]"
	} else if f.cursor >= len(f.lines)-1 {
		pos = " [bot]"
	} else {
		pct := (f.cursor * 100) / len(f.lines)
		pos = fmt.Sprintf(" [%d%%]", pct)
	}

	return f.styles.WindowTitle.Render(name + pos)
}

func (f *FileView) renderLine(lineNum, lineNumWidth, maxWidth int) string {
	line := f.lines[lineNum]

	// Replace tabs with spaces
	line = strings.ReplaceAll(line, "\t", "    ")

	// Line number
	lineNumStr := fmt.Sprintf("%*d", lineNumWidth, lineNum+1)
	lineNumStyled := f.styles.Muted.Render(lineNumStr + " │ ")

	// Cursor indicator
	cursor := "  "
	if lineNum == f.cursor {
		cursor = "> "
	}

	// Calculate available width for content
	prefixWidth := len(cursor) + lineNumWidth + 3 // cursor + linenum + " │ "
	availableWidth := maxWidth - prefixWidth
	if availableWidth < 1 {
		availableWidth = 1
	}

	// Truncate if needed
	if len(line) > availableWidth {
		line = line[:availableWidth-1] + "…"
	}

	// Style content
	var contentStyle lipgloss.Style
	if lineNum == f.cursor {
		contentStyle = f.styles.ListItemSelected
	} else {
		contentStyle = f.styles.ListItem
	}

	return cursor + lineNumStyled + contentStyle.Render(line)
}
