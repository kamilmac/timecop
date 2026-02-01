package app

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/atotto/clipboard"
	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/github"
	"github.com/kmacinski/blocks/internal/keys"
	"github.com/kmacinski/blocks/internal/layout"
	"github.com/kmacinski/blocks/internal/ui"
	"github.com/kmacinski/blocks/internal/watcher"
	"github.com/kmacinski/blocks/internal/window"
)

// App is the main application model
type App struct {
	state  *State
	git    git.Client
	gh     *github.Client
	layout *layout.Manager
	styles ui.Styles

	// Windows
	fileList   *window.FileList
	diffView   *window.DiffView
	commitList *window.CommitList
	help       *window.Help

	// Window registry
	windows     map[string]window.Window
	assignments map[string]string

	// Dimensions
	width  int
	height int

	// Status message
	statusMessage string

	// File watcher
	watcher *watcher.GitWatcher
	program *tea.Program
}

// New creates a new application
func New(gitClient git.Client) *App {
	styles := ui.DefaultStyles
	state := NewState()

	// Create windows
	fileList := window.NewFileList(styles)
	diffView := window.NewDiffView(styles)
	commitList := window.NewCommitList(styles)
	help := window.NewHelp(styles)

	// Set initial focus
	fileList.SetFocus(true)

	// Create window registry
	windows := map[string]window.Window{
		"filelist":   fileList,
		"diffview":   diffView,
		"commitlist": commitList,
		"help":       help,
	}

	// Default assignments for different layouts
	assignments := map[string]string{
		// ThreeSlot layout
		"left-top":    "filelist",
		"left-bottom": "commitlist",
		"right":       "diffview",
		// StackedThree layout
		"top":    "filelist",
		"middle": "diffview",
		"bottom": "commitlist",
	}

	app := &App{
		state:       state,
		git:         gitClient,
		gh:          github.NewClient(),
		layout:      layout.NewManager(layout.DefaultResponsive),
		styles:      styles,
		fileList:    fileList,
		diffView:    diffView,
		commitList:  commitList,
		help:        help,
		windows:     windows,
		assignments: assignments,
	}

	// Set file selection callback
	fileList.SetOnSelect(func(index int, path string) tea.Cmd {
		return func() tea.Msg {
			// Check if folder is selected
			if fileList.IsFolderSelected() {
				return FolderSelectedMsg{
					Path:     path,
					IsRoot:   fileList.IsRootSelected(),
					Children: fileList.SelectedChildren(),
				}
			}
			return FileSelectedMsg{Index: index, Path: path}
		}
	})

	return app
}

// SetProgram sets the tea.Program reference for sending messages from watcher
func (a *App) SetProgram(p *tea.Program) {
	a.program = p

	// Start file watcher with 500ms debounce
	w, err := watcher.New(500*time.Millisecond, func() {
		if a.program != nil {
			a.program.Send(GitChangedMsg{})
		}
	})
	if err == nil {
		a.watcher = w
		a.watcher.Start()
	}
}

// Cleanup stops the watcher
func (a *App) Cleanup() {
	if a.watcher != nil {
		a.watcher.Stop()
	}
}

// Init initializes the application
func (a *App) Init() tea.Cmd {
	return tea.Batch(
		a.loadBranchInfo(),
		a.loadFiles(),
		a.loadCommits(),
		a.loadPR(),
	)
}

// Update handles messages
func (a *App) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		a.width = msg.Width
		a.height = msg.Height
		a.layout.Resize(msg.Width, msg.Height)
		return a, nil

	case tea.KeyMsg:
		// Handle modal first
		if a.state.ActiveModal != "" {
			return a.handleModalKey(msg)
		}

		// Global keybindings
		switch {
		case key.Matches(msg, keys.DefaultKeyMap.Quit):
			return a, tea.Quit

		case key.Matches(msg, keys.DefaultKeyMap.Help):
			a.state.ToggleModal("help")
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.Refresh):
			return a, tea.Batch(a.loadFiles(), a.loadCommits(), a.loadDiffStats())

		case key.Matches(msg, keys.DefaultKeyMap.ModeWorking):
			a.state.SetDiffMode(git.DiffModeWorking)
			return a, tea.Batch(a.loadFiles(), a.loadDiff(), a.loadDiffStats())

		case key.Matches(msg, keys.DefaultKeyMap.ModeBranch):
			a.state.SetDiffMode(git.DiffModeBranch)
			return a, tea.Batch(a.loadFiles(), a.loadDiff(), a.loadDiffStats())

		case key.Matches(msg, keys.DefaultKeyMap.ViewChanged):
			a.state.SetFileViewMode(git.FileViewChanged)
			return a, a.loadFiles()

		case key.Matches(msg, keys.DefaultKeyMap.ViewAllFiles):
			a.state.SetFileViewMode(git.FileViewAll)
			return a, a.loadFiles()

		case key.Matches(msg, keys.DefaultKeyMap.ViewDocs):
			a.state.SetFileViewMode(git.FileViewDocs)
			return a, a.loadFiles()

		case key.Matches(msg, keys.DefaultKeyMap.ToggleDiffStyle):
			a.state.ToggleDiffStyle()
			a.diffView.SetStyle(a.state.DiffStyle)
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.Tab):
			a.cycleFocus(false)
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.ShiftTab):
			a.cycleFocus(true)
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.Left):
			a.focusPrev()
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.Right):
			a.focusNext()
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.Yank):
			if a.state.SelectedFile != "" {
				if err := clipboard.WriteAll(a.state.SelectedFile); err == nil {
					a.statusMessage = fmt.Sprintf("Copied: %s", a.state.SelectedFile)
				}
			}
			return a, nil

		case key.Matches(msg, keys.DefaultKeyMap.OpenEditor):
			if a.state.SelectedFile != "" {
				return a, a.openInEditor(a.state.SelectedFile)
			}
			return a, nil
		}

		// Delegate to focused window
		return a.delegateToFocused(msg)

	case FileSelectedMsg:
		a.state.SelectFile(msg.Index)
		a.state.SelectedFolder = ""
		a.state.IsRootSelected = false
		return a, a.loadDiff()

	case FolderSelectedMsg:
		a.state.SelectedFile = ""
		a.state.SelectedIndex = -1
		a.state.SelectedFolder = msg.Path
		a.state.IsRootSelected = msg.IsRoot
		a.state.FolderChildren = msg.Children
		return a, a.loadFolderContent()

	case FilesLoadedMsg:
		a.state.SetFiles(msg.Files)
		a.fileList.SetFiles(msg.Files)
		// Load diff for selected file
		if a.state.SelectedFile != "" {
			cmds = append(cmds, a.loadDiff())
		}
		return a, tea.Batch(cmds...)

	case DiffLoadedMsg:
		a.state.Diff = msg.Content
		a.diffView.SetContent(msg.Content, a.state.SelectedFile)
		return a, nil

	case CommitsLoadedMsg:
		a.state.Commits = msg.Commits
		a.commitList.SetCommits(msg.Commits)
		return a, nil

	case BranchInfoMsg:
		a.state.Branch = msg.Branch
		a.state.BaseBranch = msg.BaseBranch
		return a, nil

	case DiffStatsMsg:
		a.state.DiffAdded = msg.Added
		a.state.DiffRemoved = msg.Removed
		return a, nil

	case ErrorMsg:
		a.state.Error = msg.Err.Error()
		return a, nil

	case GitChangedMsg:
		// File system changed, refresh data
		return a, tea.Batch(a.loadBranchInfo(), a.loadFiles(), a.loadDiff(), a.loadCommits(), a.loadDiffStats())

	case PRLoadedMsg:
		if msg.Err != nil {
			a.state.PR = nil
		} else {
			a.state.PR = msg.PR
		}
		// Update windows with new PR data
		a.fileList.SetPR(a.state.PR)
		a.diffView.SetPR(a.state.PR)
		return a, nil

	case FolderDiffLoadedMsg:
		a.state.Diff = msg.Content
		a.diffView.SetFolderContent(msg.Content, msg.Path, a.state.IsRootSelected, a.state.PR)
		return a, nil
	}

	return a, nil
}

func (a *App) handleModalKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	// Always allow quit
	if key.Matches(msg, keys.DefaultKeyMap.Quit) {
		return a, tea.Quit
	}

	// Close modal on ? or Escape
	if key.Matches(msg, keys.DefaultKeyMap.Help) || key.Matches(msg, keys.DefaultKeyMap.Escape) {
		a.state.CloseModal()
		return a, nil
	}

	return a, nil
}

func (a *App) delegateToFocused(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd

	switch a.state.FocusedWindow {
	case "filelist":
		var w window.Window
		w, cmd = a.fileList.Update(msg)
		a.fileList = w.(*window.FileList)
	case "diffview":
		var w window.Window
		w, cmd = a.diffView.Update(msg)
		a.diffView = w.(*window.DiffView)
	case "commitlist":
		var w window.Window
		w, cmd = a.commitList.Update(msg)
		a.commitList = w.(*window.CommitList)
	}

	return a, cmd
}

func (a *App) cycleFocus(reverse bool) {
	windowOrder := []string{"filelist", "diffview", "commitlist"}
	a.state.CycleWindow(windowOrder, reverse)
	a.updateFocus()
}

func (a *App) focusNext() {
	a.cycleFocus(false)
}

func (a *App) focusPrev() {
	a.cycleFocus(true)
}

func (a *App) updateFocus() {
	a.fileList.SetFocus(a.state.FocusedWindow == "filelist")
	a.diffView.SetFocus(a.state.FocusedWindow == "diffview")
	a.commitList.SetFocus(a.state.FocusedWindow == "commitlist")
}

// View renders the application
func (a *App) View() string {
	if a.width == 0 || a.height == 0 {
		return "Loading..."
	}

	// Check if we're in a git repo
	if !a.git.IsRepo() {
		return a.renderError("Not a git repository", "Run blocks from within a git repository")
	}

	// Render status bar
	statusBar := a.renderStatusBar()

	// Render main layout
	mainView := a.layout.Render(a.windows, a.assignments, statusBar)

	// Render modal overlay if active
	if a.state.ActiveModal == "help" {
		mainView = a.renderWithModal(mainView, a.help)
	}

	return mainView
}

func (a *App) renderStatusBar() string {
	// Branch
	branch := a.state.Branch
	if branch == "" {
		branch = "unknown"
	}

	// Mode
	mode := fmt.Sprintf("[%s]", a.state.DiffMode.String())
	if a.state.DiffStyle == git.DiffStyleSideBySide {
		mode += " [split]"
	}
	if viewMode := a.state.FileViewMode.String(); viewMode != "" {
		mode += fmt.Sprintf(" [%s]", viewMode)
	}

	// File count
	fileCount := fmt.Sprintf("%d files", len(a.state.Files))

	// Diff stats
	stats := ""
	if a.state.DiffAdded > 0 || a.state.DiffRemoved > 0 {
		addedStyle := a.styles.DiffAdded
		removedStyle := a.styles.DiffRemoved
		stats = fmt.Sprintf("%s %s",
			addedStyle.Render(fmt.Sprintf("+%d", a.state.DiffAdded)),
			removedStyle.Render(fmt.Sprintf("-%d", a.state.DiffRemoved)),
		)
	}

	// Status message (temporary)
	statusMsg := ""
	if a.statusMessage != "" {
		statusMsg = a.styles.Muted.Render(" │ " + a.statusMessage)
		a.statusMessage = "" // Clear after showing
	}

	// Build status bar
	left := fmt.Sprintf(" %s  %s  %s", branch, mode, fileCount)
	if stats != "" {
		left += "  " + stats
	}
	left += statusMsg

	// Pad to full width
	padding := a.width - lipgloss.Width(left)
	if padding < 0 {
		padding = 0
	}

	return a.styles.StatusBar.
		Width(a.width).
		Render(left + strings.Repeat(" ", padding))
}

func (a *App) renderWithModal(background string, modal window.Window) string {
	// Calculate modal size - let content determine height
	modalWidth := min(50, a.width-4)
	modalHeight := min(26, a.height-4)

	// Render modal content
	modalContent := modal.View(modalWidth, modalHeight)

	// Center modal on screen
	return lipgloss.Place(
		a.width,
		a.height,
		lipgloss.Center,
		lipgloss.Center,
		modalContent,
	)
}

func (a *App) renderError(title, hint string) string {
	style := lipgloss.NewStyle().
		Foreground(lipgloss.Color("#f38ba8")).
		Bold(true).
		Padding(2)

	content := fmt.Sprintf("%s\n\n%s", title, a.styles.Muted.Render(hint))
	return lipgloss.Place(a.width, a.height, lipgloss.Center, lipgloss.Center, style.Render(content))
}

// Commands

func (a *App) loadBranchInfo() tea.Cmd {
	return func() tea.Msg {
		branch, _ := a.git.CurrentBranch()
		baseBranch, _ := a.git.BaseBranch()
		return BranchInfoMsg{Branch: branch, BaseBranch: baseBranch}
	}
}

func (a *App) loadFiles() tea.Cmd {
	return func() tea.Msg {
		var files []git.FileStatus
		var err error

		switch a.state.FileViewMode {
		case git.FileViewAll:
			files, err = a.git.ListAllFiles()
		case git.FileViewDocs:
			files, err = a.git.ListDocFiles()
		default:
			files, err = a.git.Status(a.state.DiffMode)
		}

		if err != nil {
			return ErrorMsg{Err: err}
		}
		return FilesLoadedMsg{Files: files}
	}
}

func (a *App) loadDiff() tea.Cmd {
	return func() tea.Msg {
		// Check if file is unchanged (in all-files or docs mode)
		if a.state.FileViewMode != git.FileViewChanged && a.state.SelectedIndex >= 0 && a.state.SelectedIndex < len(a.state.Files) {
			file := a.state.Files[a.state.SelectedIndex]
			if file.Status == git.StatusUnchanged {
				// Show file content instead of diff
				content, err := a.git.ReadFile(file.Path)
				if err != nil {
					return ErrorMsg{Err: err}
				}
				return DiffLoadedMsg{Content: content}
			}
		}

		content, err := a.git.Diff(a.state.SelectedFile, a.state.DiffMode)
		if err != nil {
			return ErrorMsg{Err: err}
		}
		return DiffLoadedMsg{Content: content}
	}
}

func (a *App) loadCommits() tea.Cmd {
	return func() tea.Msg {
		commits, err := a.git.Log()
		if err != nil {
			return ErrorMsg{Err: err}
		}
		return CommitsLoadedMsg{Commits: commits}
	}
}

func (a *App) loadDiffStats() tea.Cmd {
	return func() tea.Msg {
		added, removed, err := a.git.DiffStats(a.state.DiffMode)
		if err != nil {
			return DiffStatsMsg{Added: 0, Removed: 0}
		}
		return DiffStatsMsg{Added: added, Removed: removed}
	}
}

func (a *App) openInEditor(path string) tea.Cmd {
	editor := os.Getenv("EDITOR")
	if editor == "" {
		editor = "vim"
	}

	c := exec.Command(editor, path)
	return tea.ExecProcess(c, func(err error) tea.Msg {
		return RefreshMsg{}
	})
}

func (a *App) loadPR() tea.Cmd {
	return func() tea.Msg {
		pr, err := a.gh.GetPRForBranch()
		return PRLoadedMsg{PR: pr, Err: err}
	}
}

func (a *App) loadFolderContent() tea.Cmd {
	return func() tea.Msg {
		if a.state.IsRootSelected {
			// For root, we'll show PR summary - content is built by DiffView
			return FolderDiffLoadedMsg{Content: "", Path: ""}
		}

		// For folders, combine diffs of all children
		var combined strings.Builder
		for _, path := range a.state.FolderChildren {
			diff, err := a.git.Diff(path, a.state.DiffMode)
			if err == nil && diff != "" {
				combined.WriteString(diff)
				combined.WriteString("\n")
			}
		}
		return FolderDiffLoadedMsg{Content: combined.String(), Path: a.state.SelectedFolder}
	}
}
