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
- **Commit history** - what's been done on this branch?
- **All files** - browse the entire codebase
- **Docs** - your specs alongside the implementation (future)

## Features

### Implemented
- [x] Display list of changed files (git status)
- [x] Tree view for file list with directories
- [x] Show unified diff for selected file
- [x] Vim-style navigation (`j`/`k`)
- [x] Auto-refresh on file changes (500ms debounce via fsnotify)
- [x] Syntax highlighting for diffs (+ green, - red)
- [x] Status bar (branch, mode, file count, diff stats)
- [x] Yank path (`y` to copy file path to clipboard)
- [x] Open in editor (`o` to open in $EDITOR)
- [x] CommitList window showing branch commits
- [x] Help modal with keybindings
- [x] All files mode (`a` to toggle) - view entire repo
- [x] File content viewer for unchanged files

### Future
- [ ] FileExplorer - full project tree navigation with expand/collapse
- [x] Docs mode - filter to markdown files only (`d` key)
- [ ] Markdown rendering - render markdown with formatting
- [x] Side-by-side diff view (`s` to toggle)
- [x] PR comments - inline comments in diff view, "C" indicator on files with comments
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
| `DiffView` | Diff preview for selected file (or file content for unchanged files) |
| `CommitList` | List of commits on current branch |
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
ThreeSlot (width >= 80)           StackedThree (width < 80)
┌───────────┬───────────────┐     ┌─────────────────────────┐
│  left-top │               │     │          top            │
├───────────┤    right      │     ├─────────────────────────┤
│left-bottom│               │     │         middle          │
└───────────┴───────────────┘     ├─────────────────────────┤
                                  │         bottom          │
                                  └─────────────────────────┘
```

Window assignments:
```go
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
```

### Modal Presentation

Help window is displayed as a modal overlay, centered on screen.

```
┌───────────┬───────────────┐
│           │ ┌───────────┐ │
│  slot1    │ │   Help    │ │
├───────────┤ │  (modal)  │ │
│  slot2    │ │           │ │
│           │ └───────────┘ │
└───────────┴───────────────┘
```

## Default UI

```
┌─────────────────────┬────────────────────────────────────┐
│ Files (4)           │ Diff                               │
│ ▼ src/              │ @@ -10,6 +10,8 @@                  │
│   > main.go      M  │  func main() {                     │
│     app.go       M  │ -    oldLine()                     │
│ ▼ internal/         │ +    newLine()                     │
│   ▼ git/            │ +    anotherLine()                 │
│       git.go     A  │  }                                 │
│   README.md      M  │                                    │
├─────────────────────┤                                    │
│ Commits (2)         │                                    │
│ > abc123 Add feat...│                                    │
│   def456 Fix bug... │                                    │
├─────────────────────┴────────────────────────────────────┤
│ feature/blocks  [branch]  4 files  +127 -43              │
└──────────────────────────────────────────────────────────┘
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
- Navigation skips directory entries (cursor only on files)
- `j`/`k` for up/down, `g`/`G` for top/bottom

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
- ` ` - Unchanged (no indicator, in all files mode)

## DiffView Window

### Display Format

Unified diff with syntax highlighting:
```
@@ -10,6 +10,8 @@ func main()
 context line
-removed line
+added line
+another new line
 context line
```

Colors:
- Green (`#a6e3a1`) for additions (`+`)
- Red (`#f38ba8`) for removals (`-`)
- Cyan/Blue for `@@` headers
- Muted for file metadata (`diff`, `index`, `---`, `+++`)

### Scrolling

- `j`/`k`: scroll line by line
- `Ctrl+d`/`Ctrl+u`: half-page scroll
- `g`/`G`: top/bottom

Title shows scroll position (top/bot/percentage).

### Content

- File selected with changes → show diff
- File selected without changes (all files mode) → show file content
- No selection → empty state message

Large diffs truncated at 10,000 lines with message.

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

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down in list / scroll diff |
| `k` / `↑` | Move up in list / scroll diff |
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
| `y` | Yank (copy) file path to clipboard |
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

### CommitList Content
- On feature branch: `git log <base>..HEAD`
- On base branch: `git log origin/<branch>..HEAD` (unpushed commits)
- No remote: Recent 20 commits

## Error States

| Condition | Behavior |
|-----------|----------|
| Not a git repo | Show message: "Not a git repository" with hint |
| No changes | Show empty state: "No changes" |
| Git command fails | Show error, keep last good state |
| Base branch not found | Fall back gracefully |
| Large diffs | Truncate at 10,000 lines with message |

## Technical Stack

- **Language**: Go
- **TUI Framework**: Bubbletea
- **Styling**: Lipgloss
- **File Watching**: fsnotify
- **Git Operations**: Shell out to git CLI

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
│   │   └── config.go       # Config structs & defaults
│   ├── layout/
│   │   └── layout.go       # Layout definitions & rendering
│   ├── window/
│   │   ├── window.go       # Window interface
│   │   ├── base.go         # Common window functionality
│   │   ├── filelist.go     # FileList with tree view
│   │   ├── diffview.go     # DiffView with syntax highlighting
│   │   ├── commitlist.go   # CommitList
│   │   └── help.go         # Help modal
│   ├── git/
│   │   ├── git.go          # Types, interface, enums
│   │   └── client.go       # GitClient implementation
│   ├── watcher/
│   │   └── watcher.go      # File system watcher
│   ├── ui/
│   │   ├── styles.go       # Lipgloss styles
│   │   └── colors.go       # Color palette
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
    Diff(path string, mode DiffMode) (string, error)
    ReadFile(path string) (string, error)
    Log() ([]Commit, error)
    BaseBranch() (string, error)
    CurrentBranch() (string, error)
    DiffStats(mode DiffMode) (added, removed int, err error)
    IsRepo() bool
}
```

## State Management

Centralized state with message-based updates (Elm architecture):

```go
type State struct {
    SelectedFile  string
    SelectedIndex int
    DiffMode      DiffMode      // Working or Branch
    DiffStyle     DiffStyle     // Unified or SideBySide
    FileViewMode  FileViewMode  // Changed, All, or Docs
    Files         []FileStatus
    Diff          string
    Commits       []Commit
    Branch        string
    BaseBranch    string
    DiffAdded     int
    DiffRemoved   int
    FocusedWindow string
    ActiveModal   string
    Error         string
}
```

Message flow:
```
User Input → App.Update() → Global keys or delegate to window
    → Window returns command → App receives message
    → State update → Re-render
```

## Future: Folder Selection & PR Summary

Enable selecting folders in FileList to view aggregated content:

- **Folder highlighting** - Allow cursor to land on directories, not just files
- **Folder diff** - When folder selected, show combined diff of all changed files within
- **Root folder = PR view** - When top-level (repo root) is selected:
  - Show PR summary (title, description, author)
  - Show PR reviews and general comments (not attached to specific lines)
  - Show relevant commits for the PR
- **Scope filtering** - Selecting a subfolder filters commits to those touching that path

This transforms Blocks into a true PR review tool where you can:
1. Start at repo root to see PR overview
2. Drill into folders to scope review
3. Select individual files for line-by-line review with inline comments

## Future: Docs Integration

The workflow: write markdown specs → AI implements → review changes.

Potential features:
- **DocsList** - Window showing project docs (*.md files)
- **DocsView** - Markdown viewer in terminal
- **Spec linking** - Associate a spec with current work/branch
- **Split context** - Spec on left, implementation diff on right
