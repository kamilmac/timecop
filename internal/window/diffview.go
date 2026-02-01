package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/github"
	"github.com/kmacinski/blocks/internal/keys"
	"github.com/kmacinski/blocks/internal/ui"
)

// DiffView displays a diff
type DiffView struct {
	Base
	viewport   viewport.Model
	content    string
	filePath   string
	folderPath string // non-empty when showing folder diff
	isRoot     bool   // true when showing PR summary
	pr         *github.PRInfo
	style      git.DiffStyle
	ready      bool
	width      int
	height     int
}

// NewDiffView creates a new diff view window
func NewDiffView(styles ui.Styles) *DiffView {
	return &DiffView{
		Base:  NewBase("diffview", styles),
		style: git.DiffStyleUnified,
	}
}

// SetContent updates the diff content
func (d *DiffView) SetContent(content string, filePath string) {
	d.content = content
	d.filePath = filePath
	d.folderPath = ""
	d.isRoot = false
	if d.ready {
		styled := d.renderContent(content)
		d.viewport.SetContent(styled)
		d.viewport.GotoTop()
	}
}


// SetStyle updates the diff display style
func (d *DiffView) SetStyle(style git.DiffStyle) {
	d.style = style
	if d.ready {
		styled := d.renderContent(d.content)
		d.viewport.SetContent(styled)
	}
}

// SetPR sets the PR info for inline comments
func (d *DiffView) SetPR(pr *github.PRInfo) {
	d.pr = pr
	// Re-render if we have content
	if d.ready && d.content != "" {
		d.viewport.SetContent(d.renderContent(d.content))
	}
}

// SetFolderContent sets content for a folder or PR summary view
func (d *DiffView) SetFolderContent(content string, folderPath string, isRoot bool, pr *github.PRInfo) {
	d.content = content
	d.filePath = ""
	d.folderPath = folderPath
	d.isRoot = isRoot
	d.pr = pr

	if d.ready {
		var styled string
		if isRoot {
			styled = d.renderPRSummary()
		} else {
			styled = d.renderContent(content)
		}
		d.viewport.SetContent(styled)
		d.viewport.GotoTop()
	}
}

// Update handles input
func (d *DiffView) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !d.focused {
		return d, nil
	}

	var cmd tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, keys.DefaultKeyMap.Down):
			d.viewport.LineDown(1)
		case key.Matches(msg, keys.DefaultKeyMap.Up):
			d.viewport.LineUp(1)
		case key.Matches(msg, keys.DefaultKeyMap.HalfPgDn):
			d.viewport.HalfViewDown()
		case key.Matches(msg, keys.DefaultKeyMap.HalfPgUp):
			d.viewport.HalfViewUp()
		case key.Matches(msg, keys.DefaultKeyMap.GotoTop):
			d.viewport.GotoTop()
		case key.Matches(msg, keys.DefaultKeyMap.GotoBot):
			d.viewport.GotoBottom()
		default:
			d.viewport, cmd = d.viewport.Update(msg)
		}
	default:
		d.viewport, cmd = d.viewport.Update(msg)
	}

	return d, cmd
}

// View renders the diff view
func (d *DiffView) View(width, height int) string {
	d.width = width
	d.height = height

	var style lipgloss.Style
	if d.focused {
		style = d.styles.WindowFocused
	} else {
		style = d.styles.WindowUnfocused
	}

	// Calculate content dimensions
	contentWidth := width - 2   // borders
	contentHeight := height - 2 // borders

	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	// Initialize or resize viewport
	if !d.ready {
		d.viewport = viewport.New(contentWidth, contentHeight-1) // -1 for title
		if d.isRoot {
			d.viewport.SetContent(d.renderPRSummary())
		} else {
			d.viewport.SetContent(d.renderContent(d.content))
		}
		d.ready = true
	} else if d.viewport.Width != contentWidth || d.viewport.Height != contentHeight-1 {
		d.viewport.Width = contentWidth
		d.viewport.Height = contentHeight - 1
		// Re-render content when width changes
		if d.isRoot {
			d.viewport.SetContent(d.renderPRSummary())
		} else {
			d.viewport.SetContent(d.renderContent(d.content))
		}
	}

	// Build content
	var lines []string

	// Title with scroll position
	titleText := d.getTitle()
	hasContent := d.content != "" || d.isRoot
	if hasContent {
		scrollPos := d.formatScrollPos()
		padding := max(0, contentWidth-len(titleText)-len(scrollPos)-4)
		titleText = fmt.Sprintf("%s %s %s",
			d.styles.WindowTitle.Render(titleText),
			d.styles.Muted.Render(strings.Repeat("─", padding)),
			d.styles.Muted.Render(scrollPos),
		)
	} else {
		titleText = d.styles.WindowTitle.Render(titleText)
	}
	lines = append(lines, titleText)

	// Viewport content
	if !hasContent {
		emptyMsg := d.styles.Muted.Render("Select a file to view diff")
		lines = append(lines, emptyMsg)
		// Pad remaining lines
		for len(lines) < contentHeight {
			lines = append(lines, "")
		}
	} else {
		lines = append(lines, d.viewport.View())
	}

	content := strings.Join(lines, "\n")

	return style.
		Width(contentWidth).
		Height(height - 2).
		Render(content)
}

func (d *DiffView) formatScrollPos() string {
	p := d.viewport.ScrollPercent() * 100
	if p <= 0 {
		return "top"
	}
	if p >= 100 {
		return "bot"
	}
	return fmt.Sprintf("%d%%", int(p))
}

func (d *DiffView) getTitle() string {
	if d.isRoot {
		return "PR Summary"
	}
	if d.folderPath != "" {
		return d.folderPath + "/"
	}
	return "Diff"
}

func (d *DiffView) renderPRSummary() string {
	var lines []string

	if d.pr == nil {
		lines = append(lines, d.styles.Muted.Render("No PR found for this branch"))
		lines = append(lines, "")
		lines = append(lines, d.styles.Muted.Render("Push your branch and create a PR to see summary here."))
		return strings.Join(lines, "\n")
	}

	// PR Title
	lines = append(lines, d.styles.ListItemSelected.Render(d.pr.Title))
	lines = append(lines, "")

	// PR metadata
	lines = append(lines, fmt.Sprintf("%s %s  %s %s  %s %s",
		d.styles.Muted.Render("Author:"),
		d.pr.Author,
		d.styles.Muted.Render("State:"),
		d.pr.State,
		d.styles.Muted.Render("#"),
		fmt.Sprintf("%d", d.pr.Number),
	))
	lines = append(lines, d.styles.Muted.Render(d.pr.URL))
	lines = append(lines, "")

	// Reviews
	if len(d.pr.Reviews) > 0 {
		lines = append(lines, d.styles.DiffHeader.Render("Reviews"))
		lines = append(lines, d.styles.Muted.Render(strings.Repeat("─", 40)))
		for _, review := range d.pr.Reviews {
			if review.State == "" && review.Body == "" {
				continue
			}
			stateStyle := d.styles.Muted
			switch review.State {
			case "APPROVED":
				stateStyle = d.styles.DiffAdded
			case "CHANGES_REQUESTED":
				stateStyle = d.styles.DiffRemoved
			}
			lines = append(lines, fmt.Sprintf("%s %s",
				d.styles.Bold.Render(review.Author),
				stateStyle.Render(review.State),
			))
			if review.Body != "" {
				for _, line := range strings.Split(review.Body, "\n") {
					lines = append(lines, "  "+line)
				}
			}
			lines = append(lines, "")
		}
	}

	// General comments (not attached to code)
	if len(d.pr.Comments) > 0 {
		lines = append(lines, d.styles.DiffHeader.Render("Comments"))
		lines = append(lines, d.styles.Muted.Render(strings.Repeat("─", 40)))
		for _, comment := range d.pr.Comments {
			lines = append(lines, d.styles.Bold.Render(comment.Author))
			for _, line := range strings.Split(comment.Body, "\n") {
				lines = append(lines, "  "+line)
			}
			lines = append(lines, "")
		}
	}

	// Summary of files with comments
	if len(d.pr.FileComments) > 0 {
		lines = append(lines, d.styles.DiffHeader.Render("Files with inline comments"))
		lines = append(lines, d.styles.Muted.Render(strings.Repeat("─", 40)))
		for path, comments := range d.pr.FileComments {
			lines = append(lines, fmt.Sprintf("  %s %s",
				path,
				d.styles.Muted.Render(fmt.Sprintf("(%d)", len(comments))),
			))
		}
	}

	return strings.Join(lines, "\n")
}

func (d *DiffView) renderContent(content string) string {
	if content == "" {
		return ""
	}

	// Check if content is a diff or plain file
	isDiff := d.isDiffContent(content)

	if d.style == git.DiffStyleSideBySide {
		if isDiff {
			return d.renderSideBySide(content)
		}
		return d.renderFileWithLineNumbers(content)
	}
	if isDiff {
		return d.styleUnifiedDiff(content)
	}
	return d.renderPlainFile(content)
}

// isDiffContent checks if content looks like a git diff
func (d *DiffView) isDiffContent(content string) bool {
	lines := strings.SplitN(content, "\n", 5)
	for _, line := range lines {
		if strings.HasPrefix(line, "diff --git") ||
			strings.HasPrefix(line, "@@") ||
			strings.HasPrefix(line, "--- a/") ||
			strings.HasPrefix(line, "+++ b/") {
			return true
		}
	}
	return false
}

// renderPlainFile renders file content without line numbers (unified style)
func (d *DiffView) renderPlainFile(content string) string {
	var styled []string
	lines := strings.Split(content, "\n")
	for _, line := range lines {
		styled = append(styled, d.styles.DiffContext.Render(line))
	}
	return strings.Join(styled, "\n")
}

// renderFileWithLineNumbers renders file content with line numbers (split style)
func (d *DiffView) renderFileWithLineNumbers(content string) string {
	lines := strings.Split(content, "\n")
	var result []string

	// Calculate line number width based on total lines
	numWidth := len(fmt.Sprintf("%d", len(lines)))
	if numWidth < 4 {
		numWidth = 4
	}

	for i, line := range lines {
		lineNum := i + 1
		numStr := fmt.Sprintf("%*d", numWidth, lineNum)

		// Handle tabs
		line = strings.ReplaceAll(line, "\t", "    ")

		// Truncate if needed
		maxWidth := d.viewport.Width - numWidth - 2 // -2 for " │"
		if maxWidth > 0 && len([]rune(line)) > maxWidth {
			runes := []rune(line)
			line = string(runes[:maxWidth-1]) + "…"
		}

		styledNum := d.styles.Muted.Render(numStr + " │")
		styledLine := d.styles.DiffContext.Render(line)
		result = append(result, styledNum+styledLine)
	}

	return strings.Join(result, "\n")
}

func (d *DiffView) styleUnifiedDiff(content string) string {
	if content == "" {
		return ""
	}

	// Build a map of comments by line number
	commentsByLine := make(map[int][]github.LineComment)
	if d.pr != nil && d.filePath != "" {
		for _, c := range d.pr.FileComments[d.filePath] {
			commentsByLine[c.Line] = append(commentsByLine[c.Line], c)
		}
	}

	var styled []string
	lines := strings.Split(content, "\n")
	var newLineNum int

	for _, line := range lines {
		var styledLine string

		// Track line numbers from hunk headers
		if strings.HasPrefix(line, "@@") {
			_, newLineNum = parseHunkHeader(line)
			newLineNum-- // Will be incremented below
			styledLine = d.styles.DiffHeader.Render(line)
		} else if strings.HasPrefix(line, "+") && !strings.HasPrefix(line, "+++") {
			styledLine = d.styles.DiffAdded.Render(line)
			newLineNum++
		} else if strings.HasPrefix(line, "-") && !strings.HasPrefix(line, "---") {
			styledLine = d.styles.DiffRemoved.Render(line)
			// Don't increment newLineNum for removed lines
		} else if strings.HasPrefix(line, "diff ") || strings.HasPrefix(line, "index ") ||
			strings.HasPrefix(line, "---") || strings.HasPrefix(line, "+++") {
			styledLine = d.styles.Muted.Render(line)
		} else {
			styledLine = d.styles.DiffContext.Render(line)
			newLineNum++
		}

		styled = append(styled, styledLine)

		// Add comments for this line
		if newLineNum > 0 {
			if comments, ok := commentsByLine[newLineNum]; ok {
				for _, c := range comments {
					styled = append(styled, d.renderComment(c)...)
				}
			}
		}
	}

	return strings.Join(styled, "\n")
}

// renderComment formats a PR comment for display in the diff
func (d *DiffView) renderComment(c github.LineComment) []string {
	var lines []string
	width := d.viewport.Width - 6 // Account for prefix
	if width < 20 {
		width = 60
	}

	// Author header
	header := fmt.Sprintf("   ┌─ %s", c.Author)
	lines = append(lines, d.styles.DiffHeader.Render(header))

	// Comment body - wrap long lines
	bodyLines := strings.Split(c.Body, "\n")
	for _, bl := range bodyLines {
		bl = strings.TrimSpace(bl)
		if bl == "" {
			lines = append(lines, d.styles.Muted.Render("   │"))
			continue
		}
		// Word wrap
		wrapped := wrapText(bl, width)
		for _, w := range wrapped {
			lines = append(lines, d.styles.Muted.Render("   │ "+w))
		}
	}

	// Footer
	lines = append(lines, d.styles.DiffHeader.Render("   └─"))

	return lines
}

// wrapText wraps text to the specified width
func wrapText(text string, width int) []string {
	if width <= 0 {
		return []string{text}
	}
	var lines []string
	words := strings.Fields(text)
	if len(words) == 0 {
		return []string{}
	}

	current := words[0]
	for _, word := range words[1:] {
		if len(current)+1+len(word) <= width {
			current += " " + word
		} else {
			lines = append(lines, current)
			current = word
		}
	}
	if current != "" {
		lines = append(lines, current)
	}
	return lines
}

// diffLine represents a line in the side-by-side view
type diffLine struct {
	leftNum   int
	leftText  string
	leftType  lineType
	rightNum  int
	rightText string
	rightType lineType
}

type lineType int

const (
	lineContext lineType = iota
	lineAdded
	lineRemoved
	lineEmpty
)

func (d *DiffView) renderSideBySide(content string) string {
	// Minimum width for side-by-side view
	minWidth := 60
	if d.viewport.Width < minWidth {
		// Fall back to unified view if too narrow
		return d.styleUnifiedDiff(content)
	}

	// Build comments map by line number
	commentsByLine := make(map[int][]github.LineComment)
	if d.pr != nil && d.filePath != "" {
		if comments, ok := d.pr.FileComments[d.filePath]; ok {
			for _, c := range comments {
				commentsByLine[c.Line] = append(commentsByLine[c.Line], c)
			}
		}
	}

	lines := strings.Split(content, "\n")
	var result []string

	// Calculate pane width (half of available space minus separator)
	paneWidth := (d.viewport.Width - 3) / 2 // 3 for " │ " separator
	if paneWidth < 20 {
		paneWidth = 20
	}

	// Number column width
	numWidth := 4

	// Process the diff
	var leftNum, rightNum int
	var i int

	for i < len(lines) {
		line := lines[i]

		// Header lines (diff, index, ---, +++)
		if strings.HasPrefix(line, "diff ") || strings.HasPrefix(line, "index ") ||
			strings.HasPrefix(line, "---") || strings.HasPrefix(line, "+++") {
			// Render header across full width
			result = append(result, d.styles.Muted.Render(truncateOrPad(line, d.viewport.Width)))
			i++
			continue
		}

		// Hunk header
		if strings.HasPrefix(line, "@@") {
			// Parse line numbers from @@ -old,len +new,len @@
			leftNum, rightNum = parseHunkHeader(line)
			result = append(result, d.styles.DiffHeader.Render(truncateOrPad(line, d.viewport.Width)))
			i++
			continue
		}

		// Context line
		if len(line) == 0 || (len(line) > 0 && line[0] != '+' && line[0] != '-') {
			text := line
			if len(text) > 0 {
				text = text[1:] // Remove leading space if present
			}
			left := d.formatSideLine(leftNum, text, lineContext, numWidth, paneWidth)
			right := d.formatSideLine(rightNum, text, lineContext, numWidth, paneWidth)
			result = append(result, left+d.styles.Muted.Render(" │ ")+right)
			// Add comments for this line
			if comments, ok := commentsByLine[rightNum]; ok {
				for _, c := range comments {
					result = append(result, d.renderComment(c)...)
				}
			}
			leftNum++
			rightNum++
			i++
			continue
		}

		// Collect consecutive - and + lines for pairing
		var removals []string
		var additions []string

		for i < len(lines) && len(lines[i]) > 0 && lines[i][0] == '-' && !strings.HasPrefix(lines[i], "---") {
			removals = append(removals, lines[i][1:])
			i++
		}
		for i < len(lines) && len(lines[i]) > 0 && lines[i][0] == '+' && !strings.HasPrefix(lines[i], "+++") {
			additions = append(additions, lines[i][1:])
			i++
		}

		// Pair up removals and additions
		maxLen := max(len(removals), len(additions))
		for j := 0; j < maxLen; j++ {
			var left, right string
			var currentRightNum int

			if j < len(removals) {
				left = d.formatSideLine(leftNum, removals[j], lineRemoved, numWidth, paneWidth)
				leftNum++
			} else {
				left = d.formatSideLine(0, "", lineEmpty, numWidth, paneWidth)
			}

			if j < len(additions) {
				currentRightNum = rightNum
				right = d.formatSideLine(rightNum, additions[j], lineAdded, numWidth, paneWidth)
				rightNum++
			} else {
				right = d.formatSideLine(0, "", lineEmpty, numWidth, paneWidth)
			}

			result = append(result, left+d.styles.Muted.Render(" │ ")+right)

			// Add comments for the new line
			if currentRightNum > 0 {
				if comments, ok := commentsByLine[currentRightNum]; ok {
					for _, c := range comments {
						result = append(result, d.renderComment(c)...)
					}
				}
			}
		}
	}

	return strings.Join(result, "\n")
}

func (d *DiffView) formatSideLine(num int, text string, lt lineType, numWidth, paneWidth int) string {
	// Format: "1234 text..."
	textWidth := paneWidth - numWidth - 1 // -1 for space after number

	var numStr string
	if num > 0 {
		numStr = fmt.Sprintf("%*d", numWidth, num)
	} else {
		numStr = strings.Repeat(" ", numWidth)
	}

	// Truncate or pad text
	displayText := truncateOrPad(text, textWidth)

	fullLine := numStr + " " + displayText

	switch lt {
	case lineAdded:
		return d.styles.DiffAdded.Render(fullLine)
	case lineRemoved:
		return d.styles.DiffRemoved.Render(fullLine)
	case lineEmpty:
		return d.styles.Muted.Render(fullLine)
	default:
		return d.styles.DiffContext.Render(fullLine)
	}
}

func parseHunkHeader(line string) (oldStart, newStart int) {
	// Parse @@ -10,6 +10,8 @@ format
	// Returns starting line numbers for old and new
	oldStart, newStart = 1, 1

	parts := strings.Split(line, " ")
	for _, p := range parts {
		if strings.HasPrefix(p, "-") && len(p) > 1 {
			fmt.Sscanf(p, "-%d", &oldStart)
		}
		if strings.HasPrefix(p, "+") && len(p) > 1 {
			fmt.Sscanf(p, "+%d", &newStart)
		}
	}
	return oldStart, newStart
}

func truncateOrPad(s string, width int) string {
	if width <= 0 {
		return ""
	}

	// Handle tabs by converting to spaces
	s = strings.ReplaceAll(s, "\t", "    ")

	runeCount := len([]rune(s))
	if runeCount > width {
		runes := []rune(s)
		if width > 1 {
			return string(runes[:width-1]) + "…"
		}
		return string(runes[:width])
	}
	return s + strings.Repeat(" ", width-runeCount)
}
