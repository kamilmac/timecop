package app

import (
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/github"
)

// FileSelectedMsg is sent when a file is selected in the file list
type FileSelectedMsg struct {
	Index int
	Path  string
}

// FilesLoadedMsg is sent when files are loaded from git
type FilesLoadedMsg struct {
	Files []git.FileStatus
}

// DiffLoadedMsg is sent when a diff is loaded
type DiffLoadedMsg struct {
	Content string
}

// BranchInfoMsg is sent with branch information
type BranchInfoMsg struct {
	Branch     string
	BaseBranch string
}

// DiffStatsMsg is sent with diff statistics
type DiffStatsMsg struct {
	Added   int
	Removed int
}

// ErrorMsg is sent when an error occurs
type ErrorMsg struct {
	Err error
}

// RefreshMsg triggers a refresh of all data
type RefreshMsg struct{}

// YankMsg is sent when a path should be copied to clipboard
type YankMsg struct {
	Path string
}

// OpenEditorMsg is sent when a file should be opened in editor
type OpenEditorMsg struct {
	Path string
}

// ToggleModalMsg toggles a modal
type ToggleModalMsg struct {
	Name string
}

// WindowResizeMsg is sent when the terminal is resized
type WindowResizeMsg struct {
	Width  int
	Height int
}

// CloseModalMsg is sent when a modal should be closed
type CloseModalMsg struct{}

// GitChangedMsg is sent when git repository changes are detected
type GitChangedMsg struct{}

// PRLoadedMsg is sent when PR info is loaded
type PRLoadedMsg struct {
	PR  *github.PRInfo
	Err error
}

// FolderSelectedMsg is sent when a folder is selected
type FolderSelectedMsg struct {
	Path     string   // folder path (empty for root)
	IsRoot   bool     // true if root/PR summary view
	Children []string // child file paths
}

// FolderDiffLoadedMsg is sent when a combined folder diff is loaded
type FolderDiffLoadedMsg struct {
	Content string
	Path    string
}

// PRPollTickMsg triggers a PR data refresh
type PRPollTickMsg struct{}
