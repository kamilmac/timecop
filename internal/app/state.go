package app

import "github.com/kmacinski/blocks/internal/git"

// State holds the shared application state
type State struct {
	// Selection
	SelectedFile  string
	SelectedIndex int
	DiffMode      git.DiffMode
	DiffStyle     git.DiffStyle
	FileViewMode  git.FileViewMode

	// Data
	Files       []git.FileStatus
	Diff        string
	Commits     []git.Commit
	Branch      string
	BaseBranch  string
	DiffAdded   int
	DiffRemoved int

	// UI
	FocusedWindow string
	ActiveModal   string // empty if no modal

	// Errors
	Error string
}

// NewState creates a new state with defaults
func NewState() *State {
	return &State{
		DiffMode:      git.DiffModeBranch,
		FocusedWindow: "filelist",
	}
}

// SelectFile updates the selected file
func (s *State) SelectFile(index int) {
	s.SelectedIndex = index
	if index >= 0 && index < len(s.Files) {
		s.SelectedFile = s.Files[index].Path
	} else {
		s.SelectedFile = ""
	}
}

// SetDiffMode changes the diff mode and resets selection
func (s *State) SetDiffMode(mode git.DiffMode) {
	s.DiffMode = mode
	s.SelectedFile = ""
	s.SelectedIndex = 0
}

// SetFileViewMode sets the file view mode and resets selection
func (s *State) SetFileViewMode(mode git.FileViewMode) {
	s.FileViewMode = mode
	s.SelectedFile = ""
	s.SelectedIndex = 0
}

// ToggleDiffStyle toggles between unified and side-by-side diff views
func (s *State) ToggleDiffStyle() {
	if s.DiffStyle == git.DiffStyleUnified {
		s.DiffStyle = git.DiffStyleSideBySide
	} else {
		s.DiffStyle = git.DiffStyleUnified
	}
}

// SetFiles updates the file list
func (s *State) SetFiles(files []git.FileStatus) {
	s.Files = files
	// Reset selection if out of bounds
	if s.SelectedIndex >= len(files) {
		s.SelectedIndex = 0
	}
	if len(files) > 0 && s.SelectedIndex < len(files) {
		s.SelectedFile = files[s.SelectedIndex].Path
	} else {
		s.SelectedFile = ""
	}
}

// ToggleModal toggles a modal on/off
func (s *State) ToggleModal(name string) {
	if s.ActiveModal == name {
		s.ActiveModal = ""
	} else {
		s.ActiveModal = name
	}
}

// CloseModal closes any open modal
func (s *State) CloseModal() {
	s.ActiveModal = ""
}

// FocusWindow sets the focused window
func (s *State) FocusWindow(name string) {
	s.FocusedWindow = name
}

// CycleWindow cycles focus to the next window
func (s *State) CycleWindow(windows []string, reverse bool) {
	if len(windows) == 0 {
		return
	}
	currentIdx := 0
	for i, w := range windows {
		if w == s.FocusedWindow {
			currentIdx = i
			break
		}
	}
	if reverse {
		currentIdx = (currentIdx - 1 + len(windows)) % len(windows)
	} else {
		currentIdx = (currentIdx + 1) % len(windows)
	}
	s.FocusedWindow = windows[currentIdx]
}
