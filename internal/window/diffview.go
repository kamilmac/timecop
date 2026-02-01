package window

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/key"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/kmacinski/blocks/internal/config"
	"github.com/kmacinski/blocks/internal/git"
	"github.com/kmacinski/blocks/internal/github"
)

// PreviewType represents what the preview pane should show
type PreviewType int

const (
	PreviewEmpty PreviewType = iota
	PreviewFileDiff
	PreviewFolderDiff
	PreviewFileContent
	PreviewCommitSummary
)

// PreviewContent holds all data needed to render a preview
type PreviewContent struct {
	Type       PreviewType
	Content    string      // diff or file content
	FilePath   string      // for file diff/content
	FolderPath string      // for folder diff
	Commit     *git.Commit // for commit summary
	PR         *github.PRInfo
}

// lineLocation maps a rendered line to its source file and line number
type lineLocation struct {
	filePath string
	lineNum  int
}

// DiffView displays diffs, file content, and summaries
type DiffView struct {
	Base
	viewport   viewport.Model
	preview    PreviewContent
	pr         *github.PRInfo
	ready      bool
	width      int
	height     int
	lineMap    []lineLocation
	cursor     int
	prRenderer *PRSummaryRenderer
}

// NewDiffView creates a new diff view window
func NewDiffView(styles config.Styles) *DiffView {
	return &DiffView{
		Base:       NewBase("diffview", styles),
		prRenderer: NewPRSummaryRenderer(styles),
	}
}

// SetPreview updates the preview content
func (d *DiffView) SetPreview(preview PreviewContent) {
	d.preview = preview
	d.cursor = 0
	if d.ready {
		d.viewport.SetContent(d.renderPreview())
		d.viewport.GotoTop()
	}
}

// SetPR sets the PR info for inline comments and summaries
func (d *DiffView) SetPR(pr *github.PRInfo) {
	d.pr = pr
	if d.ready && d.preview.Type != PreviewEmpty {
		d.viewport.SetContent(d.renderPreview())
	}
}

// GetSelectedLocation returns the file path and line number at cursor
func (d *DiffView) GetSelectedLocation() (filePath string, lineNum int) {
	if len(d.lineMap) == 0 {
		return d.preview.FilePath, 1
	}
	lineIdx := d.viewport.YOffset + d.cursor
	if lineIdx >= len(d.lineMap) {
		lineIdx = len(d.lineMap) - 1
	}
	if lineIdx < 0 {
		lineIdx = 0
	}
	loc := d.lineMap[lineIdx]
	if loc.filePath == "" {
		return d.preview.FilePath, loc.lineNum
	}
	return loc.filePath, loc.lineNum
}

// Update handles input
func (d *DiffView) Update(msg tea.Msg) (Window, tea.Cmd) {
	if !d.focused {
		return d, nil
	}

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch {
		case key.Matches(msg, config.DefaultKeyMap.Down):
			d.scrollDown(1)
		case key.Matches(msg, config.DefaultKeyMap.Up):
			d.scrollUp(1)
		case key.Matches(msg, config.DefaultKeyMap.FastDown):
			d.scrollDown(5)
		case key.Matches(msg, config.DefaultKeyMap.FastUp):
			d.scrollUp(5)
		case key.Matches(msg, config.DefaultKeyMap.HalfPgDn):
			d.scrollDown(d.viewport.Height / 2)
		case key.Matches(msg, config.DefaultKeyMap.HalfPgUp):
			d.scrollUp(d.viewport.Height / 2)
		case key.Matches(msg, config.DefaultKeyMap.GotoTop):
			d.viewport.GotoTop()
			d.cursor = 0
		case key.Matches(msg, config.DefaultKeyMap.GotoBot):
			d.viewport.GotoBottom()
			d.cursor = min(d.viewport.Height-1, d.viewport.TotalLineCount()-1)
		}
	}

	return d, nil
}

func (d *DiffView) scrollDown(lines int) {
	maxCursor := min(d.viewport.Height-1, d.viewport.TotalLineCount()-d.viewport.YOffset-1)
	if d.cursor < maxCursor {
		d.cursor = min(d.cursor+lines, maxCursor)
	} else {
		d.viewport.LineDown(lines)
	}
}

func (d *DiffView) scrollUp(lines int) {
	if d.cursor > 0 {
		d.cursor = max(d.cursor-lines, 0)
	} else {
		d.viewport.LineUp(lines)
	}
}

// View renders the diff view
func (d *DiffView) View(width, height int) string {
	d.width = width
	d.height = height

	style := d.styles.WindowUnfocused
	if d.focused {
		style = d.styles.WindowFocused
	}

	contentWidth := width - 2
	contentHeight := height - 2
	if contentWidth < 1 || contentHeight < 1 {
		return ""
	}

	// Initialize or resize viewport
	if !d.ready {
		d.viewport = viewport.New(contentWidth, contentHeight-1)
		d.viewport.SetContent(d.renderPreview())
		d.ready = true
	} else if d.viewport.Width != contentWidth || d.viewport.Height != contentHeight-1 {
		d.viewport.Width = contentWidth
		d.viewport.Height = contentHeight - 1
		d.viewport.SetContent(d.renderPreview())
	}

	var lines []string

	// Title
	titleText := d.getTitle()
	hasContent := d.preview.Type != PreviewEmpty

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

	// Content
	if !hasContent {
		lines = append(lines, d.styles.Muted.Render("Select a file to view"))
		for len(lines) < contentHeight {
			lines = append(lines, "")
		}
	} else {
		viewportContent := d.viewport.View()
		if d.focused && len(d.lineMap) > 0 {
			viewportLines := strings.Split(viewportContent, "\n")
			if d.cursor >= 0 && d.cursor < len(viewportLines) {
				cursorStyle := lipgloss.NewStyle().Reverse(true)
				viewportLines[d.cursor] = cursorStyle.Render(viewportLines[d.cursor])
			}
			viewportContent = strings.Join(viewportLines, "\n")
		}
		lines = append(lines, viewportContent)
	}

	content := strings.Join(lines, "\n")
	return style.Width(contentWidth).Height(height - 2).Render(content)
}

func (d *DiffView) getTitle() string {
	switch d.preview.Type {
	case PreviewFileDiff:
		if d.preview.FilePath != "" {
			return d.preview.FilePath
		}
		return "Diff"
	case PreviewFolderDiff:
		if d.preview.FolderPath != "" {
			return d.preview.FolderPath + "/"
		}
		return "Folder Diff"
	case PreviewFileContent:
		if d.preview.FilePath != "" {
			return d.preview.FilePath
		}
		return "File"
	case PreviewCommitSummary:
		return "Commit & PR Summary"
	default:
		return "Preview"
	}
}

func (d *DiffView) formatScrollPos() string {
	if d.viewport.TotalLineCount() == 0 {
		return ""
	}
	percent := 0
	if d.viewport.TotalLineCount() > d.viewport.Height {
		percent = (d.viewport.YOffset * 100) / (d.viewport.TotalLineCount() - d.viewport.Height)
	}
	return fmt.Sprintf("%d%%", percent)
}

func (d *DiffView) renderPreview() string {
	switch d.preview.Type {
	case PreviewCommitSummary:
		return d.renderCommitSummary()
	case PreviewFileDiff, PreviewFolderDiff:
		return d.renderDiff(d.preview.Content)
	case PreviewFileContent:
		return d.renderFileContent(d.preview.Content)
	default:
		return ""
	}
}

func (d *DiffView) renderCommitSummary() string {
	d.lineMap = nil
	var lines []string

	// Commit details
	if d.preview.Commit != nil {
		c := d.preview.Commit
		lines = append(lines, d.styles.DiffHeader.Render("Commit"))
		lines = append(lines, d.styles.Muted.Render(strings.Repeat("─", 40)))
		lines = append(lines, fmt.Sprintf("%s %s", d.styles.Muted.Render("Hash:"), c.Hash))
		lines = append(lines, fmt.Sprintf("%s %s", d.styles.Muted.Render("Author:"), c.Author))
		lines = append(lines, fmt.Sprintf("%s %s", d.styles.Muted.Render("Date:"), c.Date))
		lines = append(lines, "")
		lines = append(lines, d.styles.ListItemSelected.Render(c.Subject))
		lines = append(lines, "")
		lines = append(lines, "")
	}

	// PR summary
	pr := d.preview.PR
	if pr == nil {
		pr = d.pr
	}
	lines = append(lines, d.prRenderer.Render(pr))

	return strings.Join(lines, "\n")
}

func (d *DiffView) renderDiff(content string) string {
	if content == "" {
		d.lineMap = nil
		return d.styles.Muted.Render("No changes")
	}

	if d.isDiffContent(content) {
		return d.renderSideBySide(content)
	}
	return d.renderFileWithLineNumbers(content)
}

func (d *DiffView) renderFileContent(content string) string {
	if content == "" {
		d.lineMap = nil
		return d.styles.Muted.Render("Empty file")
	}
	if d.isBinary(content) {
		d.lineMap = nil
		return d.styles.Muted.Render("Binary file")
	}
	return d.renderFileWithLineNumbers(content)
}

func (d *DiffView) isBinary(content string) bool {
	// Check first 8KB for null bytes (strong indicator of binary)
	checkLen := len(content)
	if checkLen > 8192 {
		checkLen = 8192
	}
	return strings.Contains(content[:checkLen], "\x00")
}

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

func (d *DiffView) renderFileWithLineNumbers(content string) string {
	lines := strings.Split(content, "\n")
	var result []string
	var lineMap []lineLocation

	numWidth := len(fmt.Sprintf("%d", len(lines)))
	numStyle := d.styles.Muted

	for i, line := range lines {
		lineNum := i + 1
		numStr := fmt.Sprintf("%*d", numWidth, lineNum)

		// Expand tabs
		expanded := strings.ReplaceAll(line, "\t", strings.Repeat(" ", config.DiffTabWidth))

		result = append(result, fmt.Sprintf("%s │ %s", numStyle.Render(numStr), expanded))
		lineMap = append(lineMap, lineLocation{filePath: d.preview.FilePath, lineNum: lineNum})
	}

	d.lineMap = lineMap
	return strings.Join(result, "\n")
}

func (d *DiffView) renderSideBySide(content string) string {
	lines := strings.Split(content, "\n")
	var result []string
	var lineMap []lineLocation

	paneWidth := (d.width - 2 - 3) / 2 // -2 for borders, -3 for separator
	if paneWidth < config.DiffPaneMinWidth {
		paneWidth = config.DiffPaneMinWidth
	}
	numWidth := config.DiffLineNumWidth

	var leftNum, rightNum int
	var currentFile string

	for _, line := range lines {
		// Track file headers
		if strings.HasPrefix(line, "diff --git") {
			parts := strings.Split(line, " ")
			if len(parts) >= 4 {
				currentFile = strings.TrimPrefix(parts[2], "a/")
			}
			header := d.styles.DiffHeader.Render(line)
			result = append(result, header)
			lineMap = append(lineMap, lineLocation{})
			continue
		}

		// Handle hunk headers
		if strings.HasPrefix(line, "@@") {
			leftNum, rightNum = d.parseHunkHeader(line)
			header := d.styles.DiffHeader.Render(line)
			result = append(result, header)
			lineMap = append(lineMap, lineLocation{})
			continue
		}

		// Skip other headers
		if strings.HasPrefix(line, "---") || strings.HasPrefix(line, "+++") ||
			strings.HasPrefix(line, "index ") || strings.HasPrefix(line, "new file") ||
			strings.HasPrefix(line, "deleted file") {
			result = append(result, d.styles.Muted.Render(line))
			lineMap = append(lineMap, lineLocation{})
			continue
		}

		// Render diff lines
		var left, right string
		var loc lineLocation

		if strings.HasPrefix(line, "-") {
			left = d.formatSideLine(leftNum, line[1:], d.styles.DiffRemoved, numWidth, paneWidth)
			right = strings.Repeat(" ", paneWidth)
			loc = lineLocation{filePath: currentFile, lineNum: leftNum}
			leftNum++
		} else if strings.HasPrefix(line, "+") {
			left = strings.Repeat(" ", paneWidth)
			right = d.formatSideLine(rightNum, line[1:], d.styles.DiffAdded, numWidth, paneWidth)
			loc = lineLocation{filePath: currentFile, lineNum: rightNum}
			rightNum++
		} else if strings.HasPrefix(line, " ") {
			left = d.formatSideLine(leftNum, line[1:], d.styles.DiffContext, numWidth, paneWidth)
			right = d.formatSideLine(rightNum, line[1:], d.styles.DiffContext, numWidth, paneWidth)
			loc = lineLocation{filePath: currentFile, lineNum: rightNum}
			leftNum++
			rightNum++
		} else {
			result = append(result, line)
			lineMap = append(lineMap, lineLocation{})
			continue
		}

		result = append(result, left+" │ "+right)
		lineMap = append(lineMap, loc)
	}

	d.lineMap = lineMap
	return strings.Join(result, "\n")
}

func (d *DiffView) parseHunkHeader(line string) (int, int) {
	// Parse @@ -start,count +start,count @@
	var leftStart, rightStart int
	fmt.Sscanf(line, "@@ -%d", &leftStart)
	if idx := strings.Index(line, "+"); idx != -1 {
		fmt.Sscanf(line[idx:], "+%d", &rightStart)
	}
	return leftStart, rightStart
}

func (d *DiffView) formatSideLine(num int, content string, style lipgloss.Style, numWidth, paneWidth int) string {
	numStr := fmt.Sprintf("%*d", numWidth, num)
	if num == 0 {
		numStr = strings.Repeat(" ", numWidth)
	}

	// Expand tabs
	content = strings.ReplaceAll(content, "\t", strings.Repeat(" ", config.DiffTabWidth))

	// Truncate if needed
	contentWidth := paneWidth - numWidth - 2
	if len(content) > contentWidth {
		content = content[:contentWidth-1] + "…"
	}

	// Pad to width
	padding := contentWidth - len(content)
	if padding > 0 {
		content += strings.Repeat(" ", padding)
	}

	return d.styles.Muted.Render(numStr) + " " + style.Render(content)
}
