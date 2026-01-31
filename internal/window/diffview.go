package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/keys"
	"github.com/kmacinski/blocks/internal/ui"
)

// DiffView displays a diff
type DiffView struct {
	Base
	viewport viewport.Model
	content  string
	ready    bool
	width    int
	height   int
}

// NewDiffView creates a new diff view window
func NewDiffView(styles ui.Styles) *DiffView {
	return &DiffView{
		Base: NewBase("diffview", styles),
	}
}

// SetContent updates the diff content
func (d *DiffView) SetContent(content string) {
	d.content = content
	if d.ready {
		styled := d.styleDiff(content)
		d.viewport.SetContent(styled)
		d.viewport.GotoTop()
	}
}

// Update handles input
func (d *DiffView) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !d.focused {
		return d, nil
	}

	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, keys.DefaultKeyMap.Down):
			d.viewport.LineDown(1)
		case key.Matches(msg, keys.DefaultKeyMap.Up):
			d.viewport.LineUp(1)
		case key.Matches(msg, keys.DefaultKeyMap.HalfPgDn):
			d.viewport.HalfViewDown()
		case key.Matches(msg, keys.DefaultKeyMap.HalfPgUp):
			d.viewport.HalfViewUp()
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			d.viewport.GotoTop()
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			d.viewport.GotoBottom()
		default:
			d.viewport, cmd = d.viewport.Update(msg)
		}
	default:
		d.viewport, cmd = d.viewport.Update(msg)
	}

	return d, cmd
}

// View renders the diff view
func (d *DiffView) View(width, height int) string {
	d.width = width
	d.height = height

	var style lipgloss.Style
	if d.focused {
		style = d.styles.WindowFocused
	} else {
		style = d.styles.WindowUnfocused
	}

	// Calculate content dimensions
	contentWidth := width - 2   // borders
	contentHeight := height - 2 // borders

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	// Initialize or resize viewport
	if !d.ready {
		d.viewport = viewport.New(contentWidth, contentHeight-1) // -1 for title
		d.viewport.SetContent(d.styleDiff(d.content))
		d.ready = true
	} else if d.viewport.Width != contentWidth || d.viewport.Height != contentHeight-1 {
		d.viewport.Width = contentWidth
		d.viewport.Height = contentHeight - 1
	}

	// Build content
	var lines []string

	// Title with scroll position
	titleText := "Diff"
	if d.content != "" {
		scrollPos := d.formatScrollPos()
		padding := max(0, contentWidth-len(titleText)-len(scrollPos)-4)
		titleText = fmt.Sprintf("%s %s %s",
			d.styles.WindowTitle.Render(titleText),
			d.styles.Muted.Render(strings.Repeat("─", padding)),
			d.styles.Muted.Render(scrollPos),
		)
	} else {
		titleText = d.styles.WindowTitle.Render(titleText)
	}
	lines = append(lines, titleText)

	// Viewport content
	if d.content == "" {
		emptyMsg := d.styles.Muted.Render("Select a file to view diff")
		lines = append(lines, emptyMsg)
		// Pad remaining lines
		for len(lines) < contentHeight {
			lines = append(lines, "")
		}
	} else {
		lines = append(lines, d.viewport.View())
	}

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		Height(height - 2).
		Render(content)
}

func (d *DiffView) formatScrollPos() string {
	p := d.viewport.ScrollPercent() * 100
	if p <= 0 {
		return "top"
	}
	if p >= 100 {
		return "bot"
	}
	return fmt.Sprintf("%d%%", int(p))
}

func (d *DiffView) styleDiff(content string) string {
	if content == "" {
		return ""
	}

	var styled []string
	lines := strings.Split(content, "\n")

	for _, line := range lines {
		var styledLine string
		switch {
		case strings.HasPrefix(line, "+") && !strings.HasPrefix(line, "+++"):
			styledLine = d.styles.DiffAdded.Render(line)
		case strings.HasPrefix(line, "-") && !strings.HasPrefix(line, "---"):
			styledLine = d.styles.DiffRemoved.Render(line)
		case strings.HasPrefix(line, "@@"):
			styledLine = d.styles.DiffHeader.Render(line)
		case strings.HasPrefix(line, "diff "), strings.HasPrefix(line, "index "),
			strings.HasPrefix(line, "---"), strings.HasPrefix(line, "+++"):
			styledLine = d.styles.Muted.Render(line)
		default:
			styledLine = d.styles.DiffContext.Render(line)
		}
		styled = append(styled, styledLine)
	}

	return strings.Join(styled, "\n")
}
