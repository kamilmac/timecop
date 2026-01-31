package window

import tea "github.com/charmbracelet/bubbletea"

// Window defines the interface for all window types
type Window interface {
	// Update handles input when focused
	Update(msg tea.Msg) (Window, tea.Cmd)

	// View renders the window content
	View(width, height int) string

	// Focus state
	Focused() bool
	SetFocus(bool)

	// Identity
	Name() string
}
