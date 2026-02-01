package keys

import "github.com/charmbracelet/bubbles/key"

// KeyMap defines all keybindings for the application
type KeyMap struct {
	// Navigation
	Up        key.Binding
	Down      key.Binding
	FastUp    key.Binding
	FastDown  key.Binding
	Left      key.Binding
	Right     key.Binding
	HalfPgUp  key.Binding
	HalfPgDn  key.Binding
	GotoTop   key.Binding
	GotoBot   key.Binding
	Tab       key.Binding
	ShiftTab  key.Binding
	Enter     key.Binding
	Escape    key.Binding

	// Actions
	Quit       key.Binding
	Refresh    key.Binding
	Yank       key.Binding
	OpenEditor key.Binding
	Help       key.Binding

	// Mode switching
	ModeWorking     key.Binding
	ModeBranch      key.Binding
	ViewChanged     key.Binding
	ViewAllFiles    key.Binding
	ViewDocs        key.Binding
	ToggleDiffStyle key.Binding
}

// DefaultKeyMap returns the default keybindings
var DefaultKeyMap = KeyMap{
	Up: key.NewBinding(
		key.WithKeys("k", "up"),
		key.WithHelp("j/k", "navigate"),
	),
	Down: key.NewBinding(
		key.WithKeys("j", "down"),
		key.WithHelp("j/k", "navigate"),
	),
	FastUp: key.NewBinding(
		key.WithKeys("K"),
		key.WithHelp("J/K", "fast navigate"),
	),
	FastDown: key.NewBinding(
		key.WithKeys("J"),
		key.WithHelp("J/K", "fast navigate"),
	),
	Left: key.NewBinding(
		key.WithKeys("h"),
		key.WithHelp("h", "collapse folder"),
	),
	Right: key.NewBinding(
		key.WithKeys("l"),
		key.WithHelp("l", "expand folder"),
	),
	HalfPgUp: key.NewBinding(
		key.WithKeys("ctrl+u"),
		key.WithHelp("C-u/C-d", "half page"),
	),
	HalfPgDn: key.NewBinding(
		key.WithKeys("ctrl+d"),
		key.WithHelp("C-u/C-d", "half page"),
	),
	GotoTop: key.NewBinding(
		key.WithKeys("g"),
		key.WithHelp("g/G", "top/bottom"),
	),
	GotoBot: key.NewBinding(
		key.WithKeys("G"),
		key.WithHelp("g/G", "top/bottom"),
	),
	Tab: key.NewBinding(
		key.WithKeys("tab"),
		key.WithHelp("tab", "next window"),
	),
	ShiftTab: key.NewBinding(
		key.WithKeys("shift+tab"),
		key.WithHelp("S-tab", "prev window"),
	),
	Enter: key.NewBinding(
		key.WithKeys("enter"),
		key.WithHelp("enter", "select"),
	),
	Escape: key.NewBinding(
		key.WithKeys("esc"),
		key.WithHelp("esc", "close/unfocus"),
	),
	Quit: key.NewBinding(
		key.WithKeys("q"),
		key.WithHelp("q", "quit"),
	),
	Refresh: key.NewBinding(
		key.WithKeys("r"),
		key.WithHelp("r", "refresh"),
	),
	Yank: key.NewBinding(
		key.WithKeys("y"),
		key.WithHelp("y", "copy path"),
	),
	OpenEditor: key.NewBinding(
		key.WithKeys("o"),
		key.WithHelp("o", "open in editor"),
	),
	Help: key.NewBinding(
		key.WithKeys("?"),
		key.WithHelp("?", "help"),
	),
	ModeWorking: key.NewBinding(
		key.WithKeys("1"),
		key.WithHelp("1", "working mode"),
	),
	ModeBranch: key.NewBinding(
		key.WithKeys("2"),
		key.WithHelp("2", "branch mode"),
	),
	ViewChanged: key.NewBinding(
		key.WithKeys("c"),
		key.WithHelp("c", "changed files"),
	),
	ViewAllFiles: key.NewBinding(
		key.WithKeys("a"),
		key.WithHelp("a", "all files"),
	),
	ViewDocs: key.NewBinding(
		key.WithKeys("d"),
		key.WithHelp("d", "docs only"),
	),
	ToggleDiffStyle: key.NewBinding(
		key.WithKeys("s"),
		key.WithHelp("s", "split diff"),
	),
}

// HelpBindings returns the keybindings to display in help
func HelpBindings() []key.Binding {
	return []key.Binding{
		DefaultKeyMap.Up,
		DefaultKeyMap.Left,
		DefaultKeyMap.Tab,
		DefaultKeyMap.HalfPgUp,
		DefaultKeyMap.GotoTop,
		DefaultKeyMap.Enter,
		DefaultKeyMap.Quit,
		DefaultKeyMap.Refresh,
		DefaultKeyMap.Yank,
		DefaultKeyMap.OpenEditor,
		DefaultKeyMap.Help,
		DefaultKeyMap.ModeWorking,
		DefaultKeyMap.ModeBranch,
		DefaultKeyMap.ViewChanged,
		DefaultKeyMap.ViewAllFiles,
		DefaultKeyMap.ViewDocs,
		DefaultKeyMap.ToggleDiffStyle,
	}
}
