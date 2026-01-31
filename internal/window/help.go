package window

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/ui"
)

// Help displays keybinding help
type Help struct {
	Base
}

// NewHelp creates a new help window
func NewHelp(styles ui.Styles) *Help {
	return &Help{
		Base: NewBase("help", styles),
	}
}

// Update handles input (modal keys handled by app)
func (h *Help) Update(msg tea.Msg) (Window, tea.Cmd) {
	return h, nil
}

// View renders the help content
func (h *Help) View(width, height int) string {
	var style lipgloss.Style
	if h.focused {
		style = h.styles.Modal
	} else {
		style = h.styles.Modal
	}

	contentWidth := width - 4   // padding and border
	contentHeight := height - 4 // padding and border

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	var lines []string

	// Title
	title := h.styles.ModalTitle.Render("Keybindings")
	lines = append(lines, title)
	lines = append(lines, "")

	// Keybindings
	bindings := []struct {
		key  string
		desc string
	}{
		{"j/k", "Navigate up/down"},
		{"h/l", "Switch window"},
		{"Tab", "Cycle windows"},
		{"Ctrl+d/u", "Scroll half page"},
		{"g/G", "Go to top/bottom"},
		{"", ""},
		{"1", "Working diff mode"},
		{"2", "Branch diff mode"},
		{"", ""},
		{"y", "Copy file path"},
		{"o", "Open in $EDITOR"},
		{"r", "Refresh"},
		{"?", "Toggle help"},
		{"q", "Quit"},
	}

	for _, b := range bindings {
		if b.key == "" {
			lines = append(lines, "")
			continue
		}
		keyStyle := h.styles.Bold.Copy().Width(12)
		line := fmt.Sprintf("%s %s", keyStyle.Render(b.key), h.styles.ListItem.Render(b.desc))
		lines = append(lines, line)
	}

	lines = append(lines, "")
	lines = append(lines, h.styles.Muted.Render("Press ? or Esc to close"))

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		MaxHeight(contentHeight).
		Render(content)
}
