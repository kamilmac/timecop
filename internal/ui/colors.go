package ui

import "github.com/charmbracelet/lipgloss"

// Colors defines the color palette for the application
type Colors struct {
	Added           lipgloss.Color
	Removed         lipgloss.Color
	Context         lipgloss.Color
	Header          lipgloss.Color
	BorderFocused   lipgloss.Color
	BorderUnfocused lipgloss.Color
	StatusBar       lipgloss.Color
	StatusBarText   lipgloss.Color
	Muted           lipgloss.Color
	Text            lipgloss.Color
}

// DefaultColors returns the default color palette
var DefaultColors = Colors{
	Added:           lipgloss.Color("#a6e3a1"),
	Removed:         lipgloss.Color("#f38ba8"),
	Context:         lipgloss.Color("#cdd6f4"),
	Header:          lipgloss.Color("#89b4fa"),
	BorderFocused:   lipgloss.Color("#89b4fa"),
	BorderUnfocused: lipgloss.Color("#45475a"),
	StatusBar:       lipgloss.Color("#313244"),
	StatusBarText:   lipgloss.Color("#cdd6f4"),
	Muted:           lipgloss.Color("#6c7086"),
	Text:            lipgloss.Color("#cdd6f4"),
}
