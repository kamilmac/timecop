package git

// DiffMode represents the type of diff to show
type DiffMode int

const (
	DiffModeWorking DiffMode = iota // Uncommitted changes
	DiffModeBranch                  // Changes vs base branch
)

func (m DiffMode) String() string {
	switch m {
	case DiffModeWorking:
		return "working"
	case DiffModeBranch:
		return "branch"
	default:
		return "unknown"
	}
}

// FileStatus represents a changed file and its status
type FileStatus struct {
	Path   string
	Status Status
}

// Status represents the git status of a file
type Status int

const (
	StatusModified Status = iota
	StatusAdded
	StatusDeleted
	StatusRenamed
	StatusUntracked
)

func (s Status) String() string {
	switch s {
	case StatusModified:
		return "M"
	case StatusAdded:
		return "A"
	case StatusDeleted:
		return "D"
	case StatusRenamed:
		return "R"
	case StatusUntracked:
		return "?"
	default:
		return " "
	}
}

// Commit represents a git commit
type Commit struct {
	Hash    string
	Subject string
	Author  string
	Date    string
}

// Client defines the interface for git operations
type Client interface {
	// Status returns changed files based on the diff mode
	Status(mode DiffMode) ([]FileStatus, error)

	// Diff returns the diff for a file (or all files if path is empty)
	Diff(path string, mode DiffMode) (string, error)

	// Log returns commits on current branch vs base
	Log() ([]Commit, error)

	// BaseBranch returns the detected or configured base branch
	BaseBranch() (string, error)

	// CurrentBranch returns the current branch name
	CurrentBranch() (string, error)

	// DiffStats returns total additions and deletions
	DiffStats(mode DiffMode) (added int, removed int, err error)

	// IsRepo returns true if we're in a git repository
	IsRepo() bool
}
