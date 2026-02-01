package github

import (
	"encoding/json"
	"fmt"
	"os/exec"
	"strings"
	"time"
)

// PRInfo represents a GitHub pull request
type PRInfo struct {
	Number         int                      `json:"number"`
	Title          string                   `json:"title"`
	Body           string                   `json:"body"`
	State          string                   `json:"state"`
	URL            string                   `json:"url"`
	Author         string                   `json:"author"`
	CreatedAt      time.Time                `json:"createdAt"`
	Comments       []Comment                `json:"comments"`
	Reviews        []Review                 `json:"reviews"`
	ReviewComments []ReviewComment          `json:"reviewComments"`
	FileComments   map[string][]LineComment // Comments organized by file path
}

// Comment represents a general PR comment
type Comment struct {
	Author    string    `json:"author"`
	Body      string    `json:"body"`
	CreatedAt time.Time `json:"createdAt"`
}

// Review represents a PR review
type Review struct {
	Author    string    `json:"author"`
	Body      string    `json:"body"`
	State     string    `json:"state"` // APPROVED, CHANGES_REQUESTED, COMMENTED
	CreatedAt time.Time `json:"createdAt"`
}

// ReviewComment represents a line-level review comment
type ReviewComment struct {
	Author    string    `json:"author"`
	Body      string    `json:"body"`
	Path      string    `json:"path"`
	Line      int       `json:"line"`
	Side      string    `json:"side"` // LEFT or RIGHT
	CreatedAt time.Time `json:"createdAt"`
}

// LineComment is a simplified comment for display
type LineComment struct {
	Author string
	Body   string
	Line   int
}

// Client defines the interface for GitHub operations
type Client interface {
	IsAvailable() bool
	HasRemote() bool
	GetPRForBranch() (*PRInfo, error)
}

// CLIClient wraps the gh CLI
type CLIClient struct{}

// NewClient creates a new GitHub client using the gh CLI
func NewClient() Client {
	return &CLIClient{}
}

// IsAvailable checks if gh CLI is available and authenticated
func (c *CLIClient) IsAvailable() bool {
	cmd := exec.Command("gh", "auth", "status")
	return cmd.Run() == nil
}

// HasRemote checks if the repo has a GitHub remote
func (c *CLIClient) HasRemote() bool {
	cmd := exec.Command("gh", "repo", "view", "--json", "name")
	return cmd.Run() == nil
}

// GetPRForBranch gets PR info for the current branch
func (c *CLIClient) GetPRForBranch() (*PRInfo, error) {
	cmd := exec.Command("gh", "pr", "view", "--json",
		"number,title,body,state,url,author,createdAt,comments,reviews")
	out, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	var raw struct {
		Number    int       `json:"number"`
		Title     string    `json:"title"`
		Body      string    `json:"body"`
		State     string    `json:"state"`
		URL       string    `json:"url"`
		CreatedAt time.Time `json:"createdAt"`
		Author    struct {
			Login string `json:"login"`
		} `json:"author"`
		Comments []struct {
			Author struct {
				Login string `json:"login"`
			} `json:"author"`
			Body      string    `json:"body"`
			CreatedAt time.Time `json:"createdAt"`
		} `json:"comments"`
		Reviews []struct {
			Author struct {
				Login string `json:"login"`
			} `json:"author"`
			Body      string    `json:"body"`
			State     string    `json:"state"`
			CreatedAt time.Time `json:"createdAt"`
		} `json:"reviews"`
	}

	if err := json.Unmarshal(out, &raw); err != nil {
		return nil, err
	}

	pr := &PRInfo{
		Number:    raw.Number,
		Title:     raw.Title,
		Body:      raw.Body,
		State:     raw.State,
		URL:       raw.URL,
		Author:    raw.Author.Login,
		CreatedAt: raw.CreatedAt,
	}

	for _, c := range raw.Comments {
		pr.Comments = append(pr.Comments, Comment{
			Author:    c.Author.Login,
			Body:      c.Body,
			CreatedAt: c.CreatedAt,
		})
	}

	for _, r := range raw.Reviews {
		pr.Reviews = append(pr.Reviews, Review{
			Author:    r.Author.Login,
			Body:      r.Body,
			State:     r.State,
			CreatedAt: r.CreatedAt,
		})
	}

	// Fetch line-level review comments
	reviewComments, err := c.getReviewComments(pr.Number)
	if err == nil {
		pr.ReviewComments = reviewComments
		pr.FileComments = organizeCommentsByFile(reviewComments)
	}

	return pr, nil
}

// getReviewComments fetches line-level review comments for a PR
func (c *CLIClient) getReviewComments(prNumber int) ([]ReviewComment, error) {
	cmd := exec.Command("gh", "api",
		fmt.Sprintf("repos/{owner}/{repo}/pulls/%d/comments", prNumber),
		"--jq", `.[] | {author: .user.login, body: .body, path: .path, line: .line, side: .side, createdAt: .created_at}`)
	out, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	var comments []ReviewComment
	lines := strings.Split(strings.TrimSpace(string(out)), "\n")
	for _, line := range lines {
		if line == "" {
			continue
		}
		var rc struct {
			Author    string `json:"author"`
			Body      string `json:"body"`
			Path      string `json:"path"`
			Line      int    `json:"line"`
			Side      string `json:"side"`
			CreatedAt string `json:"createdAt"`
		}
		if err := json.Unmarshal([]byte(line), &rc); err != nil {
			continue
		}
		t, _ := time.Parse(time.RFC3339, rc.CreatedAt)
		comments = append(comments, ReviewComment{
			Author:    rc.Author,
			Body:      rc.Body,
			Path:      rc.Path,
			Line:      rc.Line,
			Side:      rc.Side,
			CreatedAt: t,
		})
	}

	return comments, nil
}

// organizeCommentsByFile groups comments by file path
func organizeCommentsByFile(comments []ReviewComment) map[string][]LineComment {
	result := make(map[string][]LineComment)
	for _, c := range comments {
		result[c.Path] = append(result[c.Path], LineComment{
			Author: c.Author,
			Body:   c.Body,
			Line:   c.Line,
		})
	}
	return result
}

// FilesWithComments returns list of file paths that have comments
func (pr *PRInfo) FilesWithComments() []string {
	var files []string
	for path := range pr.FileComments {
		files = append(files, path)
	}
	return files
}
