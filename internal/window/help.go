package window

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/kmacinski/blocks/internal/config"
)

// Help displays keybinding help
type Help struct {
	Base
}

// NewHelp creates a new help window
func NewHelp(styles config.Styles) *Help {
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
	style := h.styles.Modal

	contentWidth := width - 4   // padding and border
	contentHeight := height - 4 // padding and border

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	var lines []string

	// Title
	title := h.styles.ModalTitle.Render("Blocks - AI-Native Code Review")
	lines = append(lines, title)
	lines = append(lines, "")

	// About section
	lines = append(lines, h.styles.Bold.Render("About"))
	lines = append(lines, h.styles.Muted.Render("Review AI-generated code changes. Read-first,"))
	lines = append(lines, h.styles.Muted.Render("write-never. See what changed, approve with confidence."))
	lines = append(lines, "")

	// Navigation section
	lines = append(lines, h.styles.Bold.Render("Navigation"))
	navBindings := []struct {
		key  string
		desc string
	}{
		{"j/k", "Move up/down"},
		{"J/K", "Fast move (5 lines)"},
		{"h/l", "Collapse/expand folder"},
		{"Tab", "Switch window"},
		{"Ctrl+d/u", "Scroll half page"},
		{"g/G", "Go to top/bottom"},
		{"Enter", "Select file/folder"},
	}
	for _, b := range navBindings {
		lines = append(lines, h.formatBinding(b.key, b.desc))
	}
	lines = append(lines, "")

	// Diff modes section
	lines = append(lines, h.styles.Bold.Render("Diff Modes"))
	lines = append(lines, h.styles.Muted.Render("What changes to compare:"))
	diffBindings := []struct {
		key  string
		desc string
	}{
		{"1", "Working - uncommitted only"},
		{"2", "Branch - all vs base branch"},
	}
	for _, b := range diffBindings {
		lines = append(lines, h.formatBinding(b.key, b.desc))
	}
	lines = append(lines, "")

	// File view modes section
	lines = append(lines, h.styles.Bold.Render("File Views"))
	lines = append(lines, h.styles.Muted.Render("Which files to show:"))
	viewBindings := []struct {
		key  string
		desc string
	}{
		{"c", "Changed files only"},
		{"a", "All files in repo"},
		{"d", "Docs (*.md) only"},
	}
	for _, b := range viewBindings {
		lines = append(lines, h.formatBinding(b.key, b.desc))
	}
	lines = append(lines, "")

	// Actions section
	lines = append(lines, h.styles.Bold.Render("Actions"))
	actionBindings := []struct {
		key  string
		desc string
	}{
		{"s", "Toggle side-by-side diff"},
		{"y", "Copy path (with line number)"},
		{"o", "Open in $EDITOR"},
		{"r", "Refresh"},
		{"?", "Toggle this help"},
		{"q", "Quit"},
	}
	for _, b := range actionBindings {
		lines = append(lines, h.formatBinding(b.key, b.desc))
	}
	lines = append(lines, "")

	// Tips section
	lines = append(lines, h.styles.Bold.Render("Tips"))
	lines = append(lines, h.styles.Muted.Render("• Collapse folders with h, expand with l"))
	lines = append(lines, h.styles.Muted.Render("• Collapsed folders show aggregated status"))
	lines = append(lines, h.styles.Muted.Render("• Select root (./) to see PR summary"))
	lines = append(lines, "")

	lines = append(lines, h.styles.Muted.Render("Press ? or Esc to close"))

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		Render(content)
}

// formatBinding formats a key-description pair
func (h *Help) formatBinding(key, desc string) string {
	keyStyle := h.styles.Bold.Copy().Width(12)
	return fmt.Sprintf("%s %s", keyStyle.Render(key), h.styles.ListItem.Render(desc))
}
