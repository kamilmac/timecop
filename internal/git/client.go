package git

import (
	"os/exec"
	"strconv"
	"strings"
)

// GitClient implements the Client interface using git CLI
type GitClient struct {
	baseBranch string
}

// NewClient creates a new git client
func NewClient(baseBranch string) *GitClient {
	return &GitClient{baseBranch: baseBranch}
}

// IsRepo returns true if we're in a git repository
func (c *GitClient) IsRepo() bool {
	cmd := exec.Command("git", "rev-parse", "--git-dir")
	return cmd.Run() == nil
}

// CurrentBranch returns the current branch name
func (c *GitClient) CurrentBranch() (string, error) {
	cmd := exec.Command("git", "branch", "--show-current")
	out, err := cmd.Output()
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(out)), nil
}

// BaseBranch returns the detected or configured base branch
func (c *GitClient) BaseBranch() (string, error) {
	// If explicitly configured, use that
	if c.baseBranch != "" {
		return c.baseBranch, nil
	}

	// Try git config
	cmd := exec.Command("git", "config", "init.defaultBranch")
	if out, err := cmd.Output(); err == nil {
		branch := strings.TrimSpace(string(out))
		if branch != "" && c.branchExists(branch) {
			return branch, nil
		}
	}

	// Try common names
	for _, name := range []string{"main", "master"} {
		if c.branchExists(name) {
			return name, nil
		}
	}

	// Try remote branches
	for _, name := range []string{"origin/main", "origin/master"} {
		if c.branchExists(name) {
			return name, nil
		}
	}

	return "", nil
}

func (c *GitClient) branchExists(name string) bool {
	cmd := exec.Command("git", "rev-parse", "--verify", name)
	return cmd.Run() == nil
}

// Status returns changed files based on the diff mode
func (c *GitClient) Status(mode DiffMode) ([]FileStatus, error) {
	var files []FileStatus

	switch mode {
	case DiffModeWorking:
		// Get working directory changes
		cmd := exec.Command("git", "status", "--porcelain")
		out, err := cmd.Output()
		if err != nil {
			return nil, err
		}
		files = parseStatus(string(out))

	case DiffModeBranch:
		// Get ALL changes vs base branch (committed + uncommitted)
		base, err := c.BaseBranch()
		if err != nil || base == "" {
			return nil, err
		}
		// Using two-dot diff shows working tree vs base (includes uncommitted)
		cmd := exec.Command("git", "diff", "--name-status", base)
		out, err := cmd.Output()
		if err != nil {
			// Fallback: maybe we're on the base branch itself
			cmd = exec.Command("git", "diff", "--name-status", "origin/"+base)
			out, err = cmd.Output()
			if err != nil {
				return nil, err
			}
		}
		files = parseNameStatus(string(out))
	}

	return files, nil
}

// ListAllFiles returns all tracked files with their git status
func (c *GitClient) ListAllFiles() ([]FileStatus, error) {
	// Get all tracked files
	cmd := exec.Command("git", "ls-files")
	out, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	// Get current status for changed files
	statusCmd := exec.Command("git", "status", "--porcelain")
	statusOut, _ := statusCmd.Output()
	changedFiles := make(map[string]Status)
	for _, fs := range parseStatus(string(statusOut)) {
		changedFiles[fs.Path] = fs.Status
	}

	// Build file list with status
	var files []FileStatus
	lines := strings.Split(strings.TrimSpace(string(out)), "\n")
	for _, line := range lines {
		if line == "" {
			continue
		}
		status := StatusUnchanged
		if s, ok := changedFiles[line]; ok {
			status = s
		}
		files = append(files, FileStatus{Path: line, Status: status})
	}

	// Add untracked files from status
	for path, status := range changedFiles {
		if status == StatusUntracked {
			files = append(files, FileStatus{Path: path, Status: status})
		}
	}

	return files, nil
}

func parseStatus(output string) []FileStatus {
	var files []FileStatus
	lines := strings.Split(strings.TrimSpace(output), "\n")
	for _, line := range lines {
		if len(line) < 3 {
			continue
		}
		statusCode := line[0:2]
		path := strings.TrimSpace(line[2:])

		var status Status
		switch {
		case statusCode[0] == '?' || statusCode[1] == '?':
			status = StatusUntracked
		case statusCode[0] == 'A' || statusCode[1] == 'A':
			status = StatusAdded
		case statusCode[0] == 'D' || statusCode[1] == 'D':
			status = StatusDeleted
		case statusCode[0] == 'R' || statusCode[1] == 'R':
			status = StatusRenamed
		default:
			status = StatusModified
		}

		files = append(files, FileStatus{Path: path, Status: status})
	}
	return files
}

func parseNameStatus(output string) []FileStatus {
	var files []FileStatus
	lines := strings.Split(strings.TrimSpace(output), "\n")
	for _, line := range lines {
		if line == "" {
			continue
		}
		parts := strings.Fields(line)
		if len(parts) < 2 {
			continue
		}
		statusCode := parts[0]
		path := parts[len(parts)-1] // Handle renames (last field is the new name)

		var status Status
		switch statusCode[0] {
		case 'A':
			status = StatusAdded
		case 'D':
			status = StatusDeleted
		case 'R':
			status = StatusRenamed
		default:
			status = StatusModified
		}

		files = append(files, FileStatus{Path: path, Status: status})
	}
	return files
}

// Diff returns the diff for a file (or all files if path is empty)
func (c *GitClient) Diff(path string, mode DiffMode) (string, error) {
	var args []string

	switch mode {
	case DiffModeWorking:
		args = []string{"diff"}
		if path != "" {
			args = append(args, "--", path)
		}

	case DiffModeBranch:
		base, err := c.BaseBranch()
		if err != nil || base == "" {
			return "", err
		}
		// Using base (not base...HEAD) includes uncommitted changes
		args = []string{"diff", base}
		if path != "" {
			args = append(args, "--", path)
		}
	}

	cmd := exec.Command("git", args...)
	out, err := cmd.Output()
	if err != nil {
		return "", err
	}

	// Truncate if too large
	result := string(out)
	lines := strings.Split(result, "\n")
	if len(lines) > 10000 {
		lines = lines[:10000]
		lines = append(lines, "", "[truncated - showing first 10,000 lines]")
		result = strings.Join(lines, "\n")
	}

	return result, nil
}

// ReadFile returns the content of a file
func (c *GitClient) ReadFile(path string) (string, error) {
	cmd := exec.Command("cat", path)
	out, err := cmd.Output()
	if err != nil {
		return "", err
	}

	// Truncate if too large
	result := string(out)
	lines := strings.Split(result, "\n")
	if len(lines) > 10000 {
		lines = lines[:10000]
		lines = append(lines, "", "[truncated - showing first 10,000 lines]")
		result = strings.Join(lines, "\n")
	}

	return result, nil
}

// Log returns commits on current branch vs base
func (c *GitClient) Log() ([]Commit, error) {
	base, err := c.BaseBranch()
	if err != nil {
		return nil, err
	}

	currentBranch, err := c.CurrentBranch()
	if err != nil {
		return nil, err
	}

	var args []string
	if base != "" && currentBranch != base {
		// Show commits on branch vs base
		args = []string{"log", "--oneline", "--format=%h|%s|%an|%cr", base + "..HEAD"}
	} else {
		// On base branch - show unpushed commits
		remote := "origin/" + currentBranch
		if c.branchExists(remote) {
			args = []string{"log", "--oneline", "--format=%h|%s|%an|%cr", remote + "..HEAD"}
		} else {
			// No remote, show recent commits
			args = []string{"log", "--oneline", "--format=%h|%s|%an|%cr", "-20"}
		}
	}

	cmd := exec.Command("git", args...)
	out, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	return parseLog(string(out)), nil
}

func parseLog(output string) []Commit {
	var commits []Commit
	lines := strings.Split(strings.TrimSpace(output), "\n")
	for _, line := range lines {
		if line == "" {
			continue
		}
		parts := strings.SplitN(line, "|", 4)
		if len(parts) < 4 {
			continue
		}
		commits = append(commits, Commit{
			Hash:    parts[0],
			Subject: parts[1],
			Author:  parts[2],
			Date:    parts[3],
		})
	}
	return commits
}

// DiffStats returns total additions and deletions
func (c *GitClient) DiffStats(mode DiffMode) (added int, removed int, err error) {
	var args []string

	switch mode {
	case DiffModeWorking:
		args = []string{"diff", "--shortstat"}
	case DiffModeBranch:
		base, err := c.BaseBranch()
		if err != nil || base == "" {
			return 0, 0, err
		}
		// Using base (not base...HEAD) includes uncommitted changes
		args = []string{"diff", "--shortstat", base}
	}

	cmd := exec.Command("git", args...)
	out, err := cmd.Output()
	if err != nil {
		return 0, 0, err
	}

	// Parse output like: " 5 files changed, 100 insertions(+), 50 deletions(-)"
	output := string(out)
	if strings.Contains(output, "insertion") {
		parts := strings.Split(output, ",")
		for _, part := range parts {
			part = strings.TrimSpace(part)
			if strings.Contains(part, "insertion") {
				fields := strings.Fields(part)
				if len(fields) > 0 {
					added, _ = strconv.Atoi(fields[0])
				}
			} else if strings.Contains(part, "deletion") {
				fields := strings.Fields(part)
				if len(fields) > 0 {
					removed, _ = strconv.Atoi(fields[0])
				}
			}
		}
	}

	return added, removed, nil
}
