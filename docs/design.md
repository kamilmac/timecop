# Blocks - AI-Native IDE

A read-first terminal IDE for AI-driven development workflows.

## Vision

When AI writes code, you don't need a traditional IDE. You need:
- **Visibility** - see what the AI is changing
- **Navigation** - understand the codebase
- **Review** - approve changes with confidence
- **Context** - specs and docs alongside code

This is read-heavy, not write-heavy. The human reviews, the AI writes.

## Problem

Current IDEs are built for humans writing code. In an AI workflow:
- You constantly run `git diff` to see changes
- You switch between terminal and editor
- You lose track of what changed vs your spec
- You trust blindly and review later

## Solution

A lightweight TUI that serves as your primary interface when working with AI:
- **Changed files** - what did the AI touch?
- **Diff view** - what exactly changed?
- **PR summary** - overview of the pull request
- **All files** - browse the entire codebase
- **Docs** - your specs alongside the implementation

## Features

### Implemented
- [x] Display list of changed files (git status)
- [x] Tree view for file list with directories
- [x] Show unified diff for selected file
- [x] Vim-style navigation (`j`/`k`)
- [x] Fast navigation (`J`/`K` - 5 lines at a time)
- [x] Auto-refresh on file changes (500ms debounce via fsnotify)
- [x] Syntax highlighting for diffs (+ green, - red)
- [x] Status bar (branch, mode, file count, diff stats)
- [x] Yank path (`y` to copy file path to clipboard)
- [x] Open in editor (`o` to open in $EDITOR)
- [x] Help modal with keybindings
- [x] All files mode (`a` to toggle) - view entire repo
- [x] Docs mode (`d` to toggle) - filter to markdown files only
- [x] File content viewer for unchanged files
- [x] Side-by-side diff view (`s` to toggle)
- [x] PR comments - inline comments in diff view, "C" indicator on files with comments
- [x] Folder selection - select directories to view combined diff
- [x] PR summary view - when root is selected, shows PR title, description, author
- [x] Line selection in diff view - navigate to specific lines with cursor

### Future
- [ ] FileExplorer - full project tree navigation with expand/collapse
- [ ] Markdown rendering - render markdown with formatting
- [ ] Hooks/events for integration with AI agents (Claude Code, Cursor, Aider, etc.)

## Architecture

### Terminology

| Term | Definition |
|------|------------|
| **Window** | A renderable component with its own state. Knows nothing about where it's placed. Given width/height, renders content. |
| **Slot** | A named rectangular area in a layout. Has position and dimensions. Holds one window. |
| **Layout** | Defines slot structure and arrangement. Knows nothing about window types. Handles responsive resizing. |
| **Modal** | A presentation mode. Any window can be shown as a modal (floating, overlays layout, captures input). |

### Windows

Windows implement a common interface. Layout doesn't care what type they are.

| Window | Description |
|--------|-------------|
| `FileList` | Tree view of changed files with status indicators |
| `DiffView` | Diff preview for selected file (or file content, PR summary, folder diff) |
| `Help` | Keybinding reference (modal) |

Each window:
- Has its own state (cursor position, scroll offset, etc.)
- Can be focused or unfocused
- Renders itself given width/height (doesn't know about layout)
- Handles its own key events when focused

### Modes

#### Diff Modes
Control what changes are compared:

| Mode | Key | Command | Description |
|------|-----|---------|-------------|
| Working | `1` | `git diff` | Uncommitted changes only |
| Branch | `2` | `git diff <base>` | All changes on branch vs base (including uncommitted) |

Default mode is **Branch**.

#### File View Modes
Control what files are shown (independent of diff mode):

| Mode | Key | Description |
|------|-----|-------------|
| Changed | `c` | Only files with changes (default) |
| All | `a` | All tracked files in repository |
| Docs | `d` | Markdown files only (*.md) |

When viewing all files or docs, selecting an unchanged file shows its full content instead of an empty diff.

### Layouts

Layouts define slot structure. They don't know about window types.

```
TwoColumn (width >= 80)              Stacked (width < 80)
┌───────────┬───────────────────┐    ┌─────────────────────────┐
│           │                   │    │          top            │
│   left    │      right        │    ├─────────────────────────┤
│   (30%)   │      (70%)        │    │         bottom          │
│           │                   │    │                         │
└───────────┴───────────────────┘    └─────────────────────────┘
```

Window assignments:
```go
assignments := map[string]string{
    // TwoColumn layout
    "left":  "filelist",
    "right": "diffview",
    // Stacked layout
    "top":    "filelist",
    "bottom": "diffview",
}
```

### Modal Presentation

Help window is displayed as a modal overlay, centered on screen.

```
┌───────────┬───────────────────┐
│           │  ┌─────────────┐  │
│  filelist │  │    Help     │  │
│           │  │   (modal)   │  │
│           │  │             │  │
│           │  └─────────────┘  │
└───────────┴───────────────────┘
```

## Default UI

```
┌─────────────────────┬──────────────────────────────────────┐
│ Files (4)           │ Diff                                 │
│ ▼ src/              │  func main() {                       │
│   > main.go      M  │ -    oldLine()                       │
│     app.go       M  │ +    newLine()                       │
│ ▼ internal/         │ +    anotherLine()                   │
│   ▼ git/            │  }                                   │
│       git.go     A  │                                      │
│   README.md      M  │                                      │
│                     │                                      │
├─────────────────────┴──────────────────────────────────────┤
│ feature/blocks  [branch]  4 files  +127 -43                │
└────────────────────────────────────────────────────────────┘
```

## Status Bar

Shows at-a-glance context:

```
┌──────────────────────────────────────────────────────────┐
│ feature/blocks  [branch] [split] [all]  4 files  +127 -43│
└──────────────────────────────────────────────────────────┘
  │                │       │       │       │        │
  │                │       │       │       │        └── Total diff stats
  │                │       │       │       └── File count
  │                │       │       └── All files mode indicator (when active)
  │                │       └── Side-by-side diff indicator (when active)
  │                └── Current diff mode
  └── Current branch
```

## FileList Window

### Tree View

Files are displayed as a tree with directories:
```
▼ src/
  > main.go           M
    app.go            M
▼ internal/
  ▼ git/
      git.go          A
  README.md           M
```
- Directories shown with `▼` prefix in muted style
- Cursor can land on directories for folder selection
- `j`/`k` for up/down, `J`/`K` for fast navigation (5 lines)
- `g`/`G` for top/bottom

### Content by Mode

| Diff Mode | File View | Shows |
|-----------|-----------|-------|
| Working | Changed | Files with uncommitted changes |
| Working | All | All files, status from `git status` |
| Branch | Changed | Files changed vs base branch |
| Branch | All | All files, status from branch diff |

### Status Indicators
- `M` - Modified (orange)
- `A` - Added (green)
- `D` - Deleted (red)
- `?` - Untracked (muted)
- `R` - Renamed (purple)
- `C` - Has PR comments (shown alongside status)
- ` ` - Unchanged (no indicator, in all files mode)

## DiffView Window

### Content Types

The DiffView displays different content based on selection:

| Selection | Content |
|-----------|---------|
| File with changes | Unified or side-by-side diff |
| File without changes | File content with line numbers |
| Folder | Combined diff of all changed files in folder |
| Root (no selection) | PR summary (if PR exists) or empty state |

### Display Format

Unified diff with syntax highlighting:
```
 context line
-removed line
+added line
+another new line
 context line
```

Colors:
- Green (`#a6e3a1`) for additions (`+`)
- Red (`#f38ba8`) for removals (`-`)
- Muted for context lines

Metadata lines (`@@`, `diff --git`, `index`, `---`, `+++`) are hidden by default.

### Line Selection

DiffView supports line-by-line navigation with a cursor:
- Cursor highlights the current line
- `j`/`k` moves cursor up/down one line
- `J`/`K` moves cursor 5 lines (fast navigation)
- `y` copies file path with current line number (`path/to/file.go:42`)

### Scrolling

- `j`/`k`: move cursor line by line
- `J`/`K`: move cursor 5 lines (fast navigation)
- `Ctrl+d`/`Ctrl+u`: half-page scroll
- `g`/`G`: top/bottom

Title shows scroll position (top/bot/percentage).

### Display Styles

Toggle with `s` key:

| Style | Description |
|-------|-------------|
| Unified | Traditional `git diff` output with +/- prefixes |
| Side-by-side | Two-pane view with old on left, new on right |

Side-by-side view:
- Shows line numbers on both sides
- Pairs deletions with additions when consecutive
- Falls back to unified if terminal width < 60 columns
- Status bar shows `[split]` indicator when active
- When viewing file content (no diff), shows line numbers on left

### PR Summary

When root folder is selected (or no file selected) and a PR exists for the current branch:
- Displays PR title, description, author
- Shows PR status (open, merged, closed)
- Lists general PR comments (not attached to specific lines)

Large diffs truncated at 10,000 lines with message.

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down in list / move cursor in diff |
| `k` / `↑` | Move up in list / move cursor in diff |
| `J` | Fast down (5 lines) |
| `K` | Fast up (5 lines) |
| `h` / `l` | Switch focused window |
| `Tab` / `Shift+Tab` | Cycle through windows |
| `Ctrl+d` | Scroll diff half-page down |
| `Ctrl+u` | Scroll diff half-page up |
| `g` | Go to top |
| `G` | Go to bottom |
| `Enter` | Select item |
| `Escape` | Close modal / unfocus |
| `q` | Quit |
| `r` | Refresh |
| `y` | Yank (copy) file path to clipboard (with line number in diff view) |
| `o` | Open file in $EDITOR |
| `?` | Toggle help modal |
| `1` | Working diff mode |
| `2` | Branch diff mode |
| `c` | Show changed files |
| `a` | Show all files |
| `d` | Show docs (markdown) only |
| `s` | Toggle side-by-side diff |

## CLI Arguments

```
blocks [flags] [path]

Arguments:
  path              Target directory (default: current dir)

Flags:
  -m, --mode        Start in mode: working, branch (default: branch)
  -b, --base        Base branch for branch mode (default: auto-detect)
  -h, --help        Show help
  -v, --version     Show version
```

## Auto-Refresh

File changes are detected via fsnotify watching:
- `.git/index` - staging changes
- `.git/HEAD` - branch changes
- `.git/refs/heads/` - commits
- Working directory (excluding `.git`, `node_modules`, `vendor`, `__pycache__`, hidden dirs)

Changes trigger refresh after 500ms debounce.

## Git Integration

### Base Branch Detection
1. Try configured base branch (via `--base` flag)
2. Try `git config init.defaultBranch`
3. Try common names: `main`, `master`
4. Try remotes: `origin/main`, `origin/master`

## GitHub Integration

Blocks integrates with GitHub via the `gh` CLI for PR-related features:

### PR Detection
- Automatically detects if current branch has an open PR
- Polls for updates every 60 seconds

### PR Information
When a PR exists, the following is available:
- PR title and description
- Author and status (open/merged/closed)
- Review comments and inline comments
- Comment authors and timestamps

### Inline Comments
- Files with PR comments show "C" indicator in file list
- Comments displayed inline in diff view at the relevant lines
- Comment threading supported

## Error States

| Condition | Behavior |
|-----------|----------|
| Not a git repo | Show message: "Not a git repository" with hint |
| No changes | Show empty state: "No changes" |
| Git command fails | Show error, keep last good state |
| Base branch not found | Fall back gracefully |
| Large diffs | Truncate at 10,000 lines with message |
| No `gh` CLI | PR features disabled gracefully |

## Technical Stack

- **Language**: Go
- **TUI Framework**: Bubbletea
- **Styling**: Lipgloss
- **File Watching**: fsnotify
- **Git Operations**: Shell out to git CLI
- **GitHub Operations**: Shell out to gh CLI

## Project Structure

```
blocks/
├── main.go                 # Entry point, CLI flags, app bootstrap
├── internal/
│   ├── app/
│   │   ├── app.go          # tea.Model, orchestration, Update/View
│   │   ├── state.go        # Shared state struct & transitions
│   │   └── messages.go     # All message types
│   ├── config/
│   │   └── config.go       # Centralized config: colors, styles, constants
│   ├── layout/
│   │   └── layout.go       # Layout definitions & rendering
│   ├── window/
│   │   ├── window.go       # Window interface
│   │   ├── base.go         # Common window functionality
│   │   ├── filelist.go     # FileList with tree view
│   │   ├── diffview.go     # DiffView with syntax highlighting
│   │   ├── prsummary.go    # PR summary renderer (extracted component)
│   │   └── help.go         # Help modal
│   ├── git/
│   │   ├── git.go          # Types, interface, enums
│   │   └── client.go       # GitClient implementation
│   ├── github/
│   │   └── github.go       # GitHub client for PR data
│   ├── watcher/
│   │   └── watcher.go      # File system watcher
│   └── keys/
│       └── keys.go         # Keybinding definitions
├── docs/
│   └── design.md
├── go.mod
└── go.sum
```

## Key Interfaces

```go
// window/window.go
type Window interface {
    Update(msg tea.Msg) (Window, tea.Cmd)
    View(width, height int) string
    Focused() bool
    SetFocus(bool)
    Name() string
}

// git/git.go
type Client interface {
    Status(mode DiffMode) ([]FileStatus, error)
    ListAllFiles() ([]FileStatus, error)
    ListDocFiles() ([]FileStatus, error)
    Diff(path string, mode DiffMode) (string, error)
    ReadFile(path string) (string, error)
    Log() ([]Commit, error)
    BaseBranch() (string, error)
    CurrentBranch() (string, error)
    DiffStats(mode DiffMode) (added, removed int, err error)
    IsRepo() bool
}

// github/github.go
type Client interface {
    IsAvailable() bool
    HasRemote() bool
    GetPRForBranch() (*PRInfo, error)
}
```

## State Management

Centralized state with message-based updates (Elm architecture):

```go
type State struct {
    // Selection
    SelectedFile   string
    SelectedIndex  int
    SelectedFolder string   // non-empty when folder selected
    FolderChildren []string // file paths in selected folder
    IsRootSelected bool     // true when root is selected (PR summary)
    DiffMode       DiffMode
    DiffStyle      DiffStyle
    FileViewMode   FileViewMode

    // Data
    Files       []FileStatus
    Diff        string
    Branch      string
    BaseBranch  string
    DiffAdded   int
    DiffRemoved int

    // UI
    FocusedWindow string
    ActiveModal   string

    // PR data
    PR *PRInfo

    // Errors
    Error string
}
```

Message flow:
```
User Input → App.Update() → Global keys or delegate to window
    → Window returns command → App receives message
    → State update → Re-render
```

## Configuration

All configuration is centralized in `internal/config/config.go`:

### Window/Modal Names
```go
const (
    WindowFileList = "filelist"
    WindowDiffView = "diffview"
    WindowHelp     = "help"
    ModalHelp      = "help"
)
```

### Timing
```go
const (
    PRPollInterval      = 60 * time.Second
    FileWatcherDebounce = 500 * time.Millisecond
)
```

### Layout
```go
const (
    LayoutLeftRatio  = 30  // percentage
    LayoutRightRatio = 70
    LayoutBreakpoint = 80  // columns
)
```

### Diff View
```go
const (
    DiffSideBySideMinWidth = 60
    DiffMaxLines           = 10000
    DiffTabWidth           = 4
)
```

### Colors (Catppuccin Mocha)
```go
var DefaultColors = Colors{
    Added:    "#a6e3a1",  // Green
    Removed:  "#f38ba8",  // Red
    Modified: "#fab387",  // Peach
    Renamed:  "#cba6f7",  // Mauve
    Header:   "#89b4fa",  // Blue
    Muted:    "#6c7086",  // Overlay0
    // ...
}
```

## Future: Docs Integration

The workflow: write markdown specs → AI implements → review changes.

Potential features:
- **DocsView** - Markdown viewer in terminal with rendering
- **Spec linking** - Associate a spec with current work/branch
- **Split context** - Spec on left, implementation diff on right
