package app

import "github.com/kmacinski/blocks/internal/git"

// FileSelectedMsg is sent when a file is selected in the file list
type FileSelectedMsg struct {
	Index int
	Path  string
}

// DiffModeChangedMsg is sent when the diff mode changes
type DiffModeChangedMsg struct {
	Mode git.DiffMode
}

// FilesLoadedMsg is sent when files are loaded from git
type FilesLoadedMsg struct {
	Files []git.FileStatus
}

// DiffLoadedMsg is sent when a diff is loaded
type DiffLoadedMsg struct {
	Content string
}

// CommitsLoadedMsg is sent when commits are loaded
type CommitsLoadedMsg struct {
	Commits []git.Commit
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
