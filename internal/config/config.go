package config

import (
	"time"

	"github.com/charmbracelet/lipgloss"
)

// =============================================================================
// Window and Modal Names
// =============================================================================

const (
	WindowFileList = "filelist"
	WindowDiffView = "diffview"
	WindowFileView = "fileview"
	WindowHelp     = "help"
)

const (
	ModalHelp = "help"
)

// =============================================================================
// Timing Configuration
// =============================================================================

const (
	PRPollInterval      = 60 * time.Second
	FileWatcherDebounce = 500 * time.Millisecond
)

// =============================================================================
// Layout Configuration
// =============================================================================

const (
	// Layout ratios (percentage)
	LayoutLeftRatio  = 30
	LayoutRightRatio = 70

	// Responsive breakpoint
	LayoutBreakpoint = 80 // columns - below this, use stacked layout
)

// =============================================================================
// Diff View Configuration
// =============================================================================

const (
	// Side-by-side view
	DiffSideBySideMinWidth = 60 // minimum terminal width for side-by-side
	DiffPaneMinWidth       = 20 // minimum width per pane
	DiffLineNumWidth       = 4  // width for line numbers

	// Content limits
	DiffMaxLines  = 10000 // max lines before truncation
	DiffTabWidth  = 4     // spaces per tab

	// Comment display
	DiffCommentMargin   = 6  // margin for inline comments
	DiffCommentMinWidth = 60 // fallback width for comments
)

// =============================================================================
// Modal Configuration
// =============================================================================

const (
	ModalMaxWidth  = 50
	ModalMaxHeight = 26
	ModalPadding   = 4 // border + padding on each side
)

// =============================================================================
// File Tree Configuration
// =============================================================================

const (
	TreeIndentSize = 2 // spaces per depth level
)

// Tree display characters
var (
	TreeDirPrefix  = "▼ "
	TreeFilePrefix = "  "
	TreeCursor     = ">"
	TreeNoCursor   = " "
	TreeTruncation = "..."
)

// =============================================================================
// Git Configuration
// =============================================================================

var (
	// Default branch names to check (in order of preference)
	GitDefaultBranches = []string{"main", "master"}

	// Remote branch names to check
	GitRemoteBranches = []string{"origin/main", "origin/master"}

	// Directories to exclude from file watching
	WatcherExcludeDirs = []string{"node_modules", "vendor", "__pycache__", ".git"}
)

// Git limits
const (
	GitRecentCommits = 20 // number of recent commits to show
)

// =============================================================================
// Colors
// =============================================================================

// Colors defines the color palette for the application
type Colors struct {
	// Diff colors
	Added    lipgloss.Color
	Removed  lipgloss.Color
	Modified lipgloss.Color
	Renamed  lipgloss.Color
	Context  lipgloss.Color

	// UI colors
	Header          lipgloss.Color
	BorderFocused   lipgloss.Color
	BorderUnfocused lipgloss.Color
	StatusBar       lipgloss.Color
	StatusBarText   lipgloss.Color
	Muted           lipgloss.Color
	Text            lipgloss.Color
	ModalBackground lipgloss.Color
}

// DefaultColors is the default color palette (Catppuccin Mocha)
var DefaultColors = Colors{
	// Diff colors
	Added:    lipgloss.Color("#a6e3a1"), // Green
	Removed:  lipgloss.Color("#f38ba8"), // Red
	Modified: lipgloss.Color("#fab387"), // Peach
	Renamed:  lipgloss.Color("#cba6f7"), // Mauve
	Context:  lipgloss.Color("#cdd6f4"), // Text

	// UI colors
	Header:          lipgloss.Color("#89b4fa"), // Blue
	BorderFocused:   lipgloss.Color("#89b4fa"), // Blue
	BorderUnfocused: lipgloss.Color("#45475a"), // Surface1
	StatusBar:       lipgloss.Color("#313244"), // Surface0
	StatusBarText:   lipgloss.Color("#cdd6f4"), // Text
	Muted:           lipgloss.Color("#6c7086"), // Overlay0
	Text:            lipgloss.Color("#cdd6f4"), // Text
	ModalBackground: lipgloss.Color("#1e1e2e"), // Base
}

// =============================================================================
// Styles
// =============================================================================

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
			Foreground(c.Modified),
		StatusAdded: lipgloss.NewStyle().
			Foreground(c.Added),
		StatusDeleted: lipgloss.NewStyle().
			Foreground(c.Removed),
		StatusUntracked: lipgloss.NewStyle().
			Foreground(c.Muted),
		StatusRenamed: lipgloss.NewStyle().
			Foreground(c.Renamed),

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
			Background(c.ModalBackground).
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
