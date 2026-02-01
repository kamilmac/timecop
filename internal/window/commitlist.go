package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/config"
	"github.com/kmacinski/blocks/internal/git"
)

// CommitList displays recent commits
type CommitList struct {
	Base
	commits  []git.Commit
	cursor   int
	width    int
	height   int
	onSelect func(commit git.Commit) tea.Cmd
}

// NewCommitList creates a new commit list window
func NewCommitList(styles config.Styles) *CommitList {
	return &CommitList{
		Base: NewBase("commitlist", styles),
	}
}

// SetCommits sets the commits to display
func (c *CommitList) SetCommits(commits []git.Commit) {
	c.commits = commits
	if c.cursor >= len(commits) {
		c.cursor = 0
	}
}

// SetOnSelect sets the callback for commit selection
func (c *CommitList) SetOnSelect(fn func(commit git.Commit) tea.Cmd) {
	c.onSelect = fn
}

// SelectedCommit returns the currently selected commit
func (c *CommitList) SelectedCommit() *git.Commit {
	if c.cursor >= 0 && c.cursor < len(c.commits) {
		return &c.commits[c.cursor]
	}
	return nil
}

// Update handles input
func (c *CommitList) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !c.focused {
		return c, nil
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, config.DefaultKeyMap.Down):
			if c.cursor < len(c.commits)-1 {
				c.cursor++
				return c, c.triggerSelect()
			}
		case key.Matches(msg, config.DefaultKeyMap.Up):
			if c.cursor > 0 {
				c.cursor--
				return c, c.triggerSelect()
			}
		case key.Matches(msg, config.DefaultKeyMap.Enter):
			return c, c.triggerSelect()
		}
	}

	return c, nil
}

func (c *CommitList) triggerSelect() tea.Cmd {
	if c.onSelect != nil && c.cursor >= 0 && c.cursor < len(c.commits) {
		return c.onSelect(c.commits[c.cursor])
	}
	return nil
}

// View renders the commit list
func (c *CommitList) View(width, height int) string {
	c.width = width
	c.height = height

	// Account for border
	contentWidth := width - 2
	contentHeight := height - 2

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	var lines []string

	if len(c.commits) == 0 {
		lines = append(lines, c.styles.Muted.Render("No commits"))
	} else {
		// Show commits that fit
		maxCommits := contentHeight
		if maxCommits > len(c.commits) {
			maxCommits = len(c.commits)
		}

		for i := 0; i < maxCommits; i++ {
			commit := c.commits[i]
			isSelected := c.focused && i == c.cursor
			line := c.formatCommit(commit, contentWidth, isSelected)
			lines = append(lines, line)
		}
	}

	// Pad to fill height
	for len(lines) < contentHeight {
		lines = append(lines, "")
	}

	content := strings.Join(lines[:contentHeight], "\n")

	// Apply window style
	style := c.styles.WindowUnfocused
	if c.focused {
		style = c.styles.WindowFocused
	}

	return style.
		Width(contentWidth).
		Height(contentHeight).
		Render(content)
}

// formatCommit formats a single commit line
func (c *CommitList) formatCommit(commit git.Commit, width int, selected bool) string {
	// Format: cursor hash subject
	cursor := " "
	if selected {
		cursor = config.TreeCursor
	}

	hash := c.styles.Muted.Render(commit.Hash[:7])

	// Calculate available space for subject
	cursorWidth := 2 // cursor + space
	hashWidth := 8   // 7 chars + space
	subjectWidth := width - cursorWidth - hashWidth
	if subjectWidth < 10 {
		subjectWidth = 10
	}

	subject := commit.Subject
	if lipgloss.Width(subject) > subjectWidth {
		subject = subject[:subjectWidth-3] + "..."
	}

	// Style subject based on selection
	if selected {
		subject = c.styles.ListItemSelected.Render(subject)
	}

	return fmt.Sprintf("%s %s %s", cursor, hash, subject)
}
