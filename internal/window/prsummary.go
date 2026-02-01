package window

import (
	"fmt"
	"strings"

	"github.com/kmacinski/blocks/internal/github"
	"github.com/kmacinski/blocks/internal/ui"
)

// PRSummaryRenderer handles rendering of PR summary views
type PRSummaryRenderer struct {
	styles ui.Styles
}

// NewPRSummaryRenderer creates a new PR summary renderer
func NewPRSummaryRenderer(styles ui.Styles) *PRSummaryRenderer {
	return &PRSummaryRenderer{styles: styles}
}

// Render renders a PR summary view
func (r *PRSummaryRenderer) Render(pr *github.PRInfo) string {
	var lines []string

	if pr == nil {
		lines = append(lines, r.styles.Muted.Render("No PR found for this branch"))
		lines = append(lines, "")
		lines = append(lines, r.styles.Muted.Render("Push your branch and create a PR to see summary here."))
		return strings.Join(lines, "\n")
	}

	// PR Title
	lines = append(lines, r.styles.ListItemSelected.Render(pr.Title))
	lines = append(lines, "")

	// PR metadata
	lines = append(lines, fmt.Sprintf("%s %s  %s %s  %s %s",
		r.styles.Muted.Render("Author:"),
		pr.Author,
		r.styles.Muted.Render("State:"),
		pr.State,
		r.styles.Muted.Render("#"),
		fmt.Sprintf("%d", pr.Number),
	))
	lines = append(lines, r.styles.Muted.Render(pr.URL))
	lines = append(lines, "")

	// PR Description
	if pr.Body != "" {
		lines = append(lines, r.styles.DiffHeader.Render("Description"))
		lines = append(lines, r.styles.Muted.Render(strings.Repeat("─", 40)))
		for _, line := range strings.Split(pr.Body, "\n") {
			lines = append(lines, line)
		}
		lines = append(lines, "")
	}

	// Reviews
	if len(pr.Reviews) > 0 {
		lines = append(lines, r.styles.DiffHeader.Render("Reviews"))
		lines = append(lines, r.styles.Muted.Render(strings.Repeat("─", 40)))
		for _, review := range pr.Reviews {
			if review.State == "" && review.Body == "" {
				continue
			}
			stateStyle := r.styles.Muted
			switch review.State {
			case "APPROVED":
				stateStyle = r.styles.DiffAdded
			case "CHANGES_REQUESTED":
				stateStyle = r.styles.DiffRemoved
			}
			lines = append(lines, fmt.Sprintf("%s %s",
				r.styles.Bold.Render(review.Author),
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
	if len(pr.Comments) > 0 {
		lines = append(lines, r.styles.DiffHeader.Render("Comments"))
		lines = append(lines, r.styles.Muted.Render(strings.Repeat("─", 40)))
		for _, comment := range pr.Comments {
			lines = append(lines, r.styles.Bold.Render(comment.Author))
			for _, line := range strings.Split(comment.Body, "\n") {
				lines = append(lines, "  "+line)
			}
			lines = append(lines, "")
		}
	}

	// Summary of files with comments
	if len(pr.FileComments) > 0 {
		lines = append(lines, r.styles.DiffHeader.Render("Files with inline comments"))
		lines = append(lines, r.styles.Muted.Render(strings.Repeat("─", 40)))
		for path, comments := range pr.FileComments {
			lines = append(lines, fmt.Sprintf("  %s %s",
				path,
				r.styles.Muted.Render(fmt.Sprintf("(%d)", len(comments))),
			))
		}
	}

	return strings.Join(lines, "\n")
}
