package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/keys"
	"github.com/kmacinski/blocks/internal/ui"
)

// CommitList displays a list of commits
type CommitList struct {
	Base
	commits []git.Commit
	cursor  int
	offset  int
	height  int
	width   int
}

// NewCommitList creates a new commit list window
func NewCommitList(styles ui.Styles) *CommitList {
	return &CommitList{
		Base: NewBase("commitlist", styles),
	}
}

// SetCommits updates the commit list
func (c *CommitList) SetCommits(commits []git.Commit) {
	c.commits = commits
	if c.cursor >= len(commits) {
		c.cursor = max(0, len(commits)-1)
	}
}

// Update handles input
func (c *CommitList) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !c.focused {
		return c, nil
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, keys.DefaultKeyMap.Down):
			if c.cursor < len(c.commits)-1 {
				c.cursor++
				c.ensureVisible()
			}
		case key.Matches(msg, keys.DefaultKeyMap.Up):
			if c.cursor > 0 {
				c.cursor--
				c.ensureVisible()
			}
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			c.cursor = 0
			c.offset = 0
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			c.cursor = max(0, len(c.commits)-1)
			c.ensureVisible()
		}
	}

	return c, nil
}

func (c *CommitList) ensureVisible() {
	visibleHeight := c.height - 3 // Account for border and title
	if visibleHeight < 1 {
		visibleHeight = 1
	}

	if c.cursor < c.offset {
		c.offset = c.cursor
	} else if c.cursor >= c.offset+visibleHeight {
		c.offset = c.cursor - visibleHeight + 1
	}
}

// View renders the commit list
func (c *CommitList) View(width, height int) string {
	c.width = width
	c.height = height

	var style lipgloss.Style
	if c.focused {
		style = c.styles.WindowFocused
	} else {
		style = c.styles.WindowUnfocused
	}

	// Calculate content dimensions
	contentWidth := width - 2  // borders
	contentHeight := height - 2 // borders

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	// Build content
	var lines []string

	// Title
	title := "Commits"
	if len(c.commits) > 0 {
		title = fmt.Sprintf("Commits (%d)", len(c.commits))
	}
	titleLine := c.styles.WindowTitle.Render(title)
	lines = append(lines, titleLine)
	contentHeight-- // Account for title

	if len(c.commits) == 0 {
		emptyMsg := c.styles.Muted.Render("No commits")
		lines = append(lines, emptyMsg)
	} else {
		// Render visible commits
		for i := c.offset; i < len(c.commits) && i < c.offset+contentHeight; i++ {
			commit := c.commits[i]
			line := c.renderCommitLine(commit, i == c.cursor, contentWidth)
			lines = append(lines, line)
		}
	}

	// Pad remaining lines
	for len(lines) < height-2 {
		lines = append(lines, "")
	}

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		Height(height - 2).
		Render(content)
}

func (c *CommitList) renderCommitLine(commit git.Commit, selected bool, maxWidth int) string {
	// Hash
	hash := c.styles.Muted.Render(commit.Hash)

	// Subject - truncate if needed
	subject := commit.Subject
	availableWidth := maxWidth - 10 // hash + spaces
	if len(subject) > availableWidth {
		subject = subject[:availableWidth-3] + "..."
	}

	var subjectStyle lipgloss.Style
	if selected {
		subjectStyle = c.styles.ListItemSelected
	} else {
		subjectStyle = c.styles.ListItem
	}

	// Selection indicator
	cursor := " "
	if selected {
		cursor = ">"
	}

	return fmt.Sprintf("%s %s %s", cursor, hash, subjectStyle.Render(subject))
}
