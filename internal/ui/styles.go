package ui

import "github.com/charmbracelet/lipgloss"

// Styles holds all the lipgloss styles for the application
type Styles struct {
	// Window styles
	WindowFocused   lipgloss.Style
	WindowUnfocused lipgloss.Style
	WindowTitle     lipgloss.Style

	// Diff styles
	DiffAdded   lipgloss.Style
	DiffRemoved lipgloss.Style
	DiffContext lipgloss.Style
	DiffHeader  lipgloss.Style

	// List styles
	ListItem         lipgloss.Style
	ListItemSelected lipgloss.Style
	ListItemMuted    lipgloss.Style

	// Status indicators
	StatusModified  lipgloss.Style
	StatusAdded     lipgloss.Style
	StatusDeleted   lipgloss.Style
	StatusUntracked lipgloss.Style
	StatusRenamed   lipgloss.Style

	// Status bar
	StatusBar     lipgloss.Style
	StatusBarItem lipgloss.Style

	// Modal
	Modal      lipgloss.Style
	ModalTitle lipgloss.Style

	// General
	Muted lipgloss.Style
	Bold  lipgloss.Style
}

// NewStyles creates a new Styles instance with the given colors
func NewStyles(c Colors) Styles {
	return Styles{
		// Window styles
		WindowFocused: lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(c.BorderFocused),
		WindowUnfocused: lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(c.BorderUnfocused),
		WindowTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(c.Header).
			Padding(0, 1),

		// Diff styles
		DiffAdded: lipgloss.NewStyle().
			Foreground(c.Added),
		DiffRemoved: lipgloss.NewStyle().
			Foreground(c.Removed),
		DiffContext: lipgloss.NewStyle().
			Foreground(c.Context),
		DiffHeader: lipgloss.NewStyle().
			Foreground(c.Header).
			Bold(true),

		// List styles
		ListItem: lipgloss.NewStyle().
			Foreground(c.Text),
		ListItemSelected: lipgloss.NewStyle().
			Foreground(c.Header).
			Bold(true),
		ListItemMuted: lipgloss.NewStyle().
			Foreground(c.Muted),

		// Status indicators
		StatusModified: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#fab387")),
		StatusAdded: lipgloss.NewStyle().
			Foreground(c.Added),
		StatusDeleted: lipgloss.NewStyle().
			Foreground(c.Removed),
		StatusUntracked: lipgloss.NewStyle().
			Foreground(c.Muted),
		StatusRenamed: lipgloss.NewStyle().
			Foreground(lipgloss.Color("#cba6f7")),

		// Status bar
		StatusBar: lipgloss.NewStyle().
			Background(c.StatusBar).
			Foreground(c.StatusBarText).
			Padding(0, 1),
		StatusBarItem: lipgloss.NewStyle().
			Foreground(c.StatusBarText).
			Padding(0, 1),

		// Modal
		Modal: lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(c.BorderFocused).
			Padding(1, 2),
		ModalTitle: lipgloss.NewStyle().
			Bold(true).
			Foreground(c.Header).
			MarginBottom(1),

		// General
		Muted: lipgloss.NewStyle().
			Foreground(c.Muted),
		Bold: lipgloss.NewStyle().
			Bold(true),
	}
}

// DefaultStyles returns styles with the default color palette
var DefaultStyles = NewStyles(DefaultColors)
