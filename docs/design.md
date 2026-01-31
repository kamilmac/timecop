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
- **Docs** - your specs alongside the implementation (future)

## Core Features

### MVP
- Display list of changed files (git status)
- Show unified diff for selected file
- Vim-style navigation (`j`/`k`)
- Auto-refresh on file changes
- Syntax highlighting for diffs (+ green, - red)
- Quit with `q`

### Near-term
- Status bar (branch, mode, file count)
- Yank path (`y` to copy file path to clipboard)
- Open in editor (`o` to open in $EDITOR)
- Tree view for file list

### Future
- FileExplorer - full project tree navigation
- Docs integration - view/navigate markdown specs alongside code changes
- Side-by-side diff view
- Hooks/events for integration with AI agents (Claude Code, Cursor, Aider, etc.)

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
| `FileList` | List of changed files with status |
| `DiffView` | Diff preview for selected file |
| `CommitList` | List of commits on current branch |
| `Help` | Keybinding reference |

Each window:
- Has its own state (cursor position, scroll offset, etc.)
- Can be focused or unfocused
- Renders itself given width/height (doesn't know about layout)
- Handles its own key events when focused

### Diff Modes
The diff view can operate in different modes:

| Mode | Command | Description |
|------|---------|-------------|
| Working | `git diff` | Uncommitted changes (what AI just did) |
| Branch | `git diff master...HEAD` | All changes on branch vs master |

Switch modes with `1` (working) or `2` (branch).

The `FileList` window adapts based on mode:
- **Working**: Shows files with uncommitted changes
- **Branch**: Shows all files changed on branch vs master

### Layouts

Layouts define slot structure. They don't know about window types.

```
2-slot layout                   3-slot layout (stacked left)
┌───────────┬───────────────┐   ┌───────────┬───────────────┐
│           │               │   │   slot1   │               │
│   slot1   │    slot2      │   ├───────────┤    slot2      │
│           │               │   │   slot3   │               │
└───────────┴───────────────┘   └───────────┴───────────────┘

Stacked layout
┌─────────────────────────┐
│         slot1           │
├─────────────────────────┤
│         slot2           │
└─────────────────────────┘
```

Window assignment is separate:
```go
// Example: assign windows to slots
assignments := map[string]string{
    "slot1": "FileList",
    "slot2": "DiffView",
    "slot3": "CommitList",  // only used if slot exists
}
```

### Layout Definition

```go
type Layout struct {
    Name      string
    Direction string  // "horizontal" or "vertical"
    Slots     []Slot
    Ratios    []int   // size ratios for slots
}

type Slot struct {
    Name      string
    // If slot contains nested slots:
    Direction string
    Children  []Slot
    Ratios    []int
}
```

**Predefined layouts:**

```go
var TwoColumn = Layout{
    Name:      "two-column",
    Direction: "horizontal",
    Ratios:    []int{30, 70},
    Slots: []Slot{
        {Name: "left"},
        {Name: "right"},
    },
}

var ThreeSlot = Layout{
    Name:      "three-slot",
    Direction: "horizontal",
    Ratios:    []int{30, 70},
    Slots: []Slot{
        {
            Name:      "left",
            Direction: "vertical",
            Ratios:    []int{60, 40},
            Children: []Slot{
                {Name: "left-top"},
                {Name: "left-bottom"},
            },
        },
        {Name: "right"},
    },
}

var Stacked = Layout{
    Name:      "stacked",
    Direction: "vertical",
    Ratios:    []int{30, 70},
    Slots: []Slot{
        {Name: "top"},
        {Name: "bottom"},
    },
}
```

### Modal Presentation

Modal is a way to display any window floating over the layout.

```
┌───────────┬───────────────┐
│           │ ┌───────────┐ │
│  slot1    │ │  Window   │ │
├───────────┤ │  (modal)  │ │
│  slot2    │ │           │ │
│           │ └───────────┘ │
└───────────┴───────────────┘
```

```go
type ModalState struct {
    Active  bool
    Window  Window  // any window
    Width   int     // percentage of screen
    Height  int     // percentage of screen
}
```

Example: `?` opens Help window as a modal. Same Help window could theoretically be assigned to a slot instead.

### Responsive Layouts

Layouts switch based on terminal size via breakpoints.

```go
type ResponsiveConfig struct {
    Breakpoints []Breakpoint
}

type Breakpoint struct {
    MinWidth int    // 0 means no minimum
    Layout   string // layout name to use
}
```

Example:
```go
var Responsive = ResponsiveConfig{
    Breakpoints: []Breakpoint{
        {MinWidth: 120, Layout: "three-slot"},
        {MinWidth: 80,  Layout: "two-column"},
        {MinWidth: 0,   Layout: "stacked"},
    },
}
```

```
Wide (120+)              Medium (80-119)         Narrow (<80)
┌───────┬───────────┐    ┌───────┬─────────┐    ┌─────────────┐
│ left  │           │    │       │         │    │    top      │
│ top   │   right   │    │ left  │  right  │    ├─────────────┤
├───────┤           │    │       │         │    │   bottom    │
│ left  │           │    │       │         │    │             │
│bottom │           │    │       │         │    │             │
└───────┴───────────┘    └───────┴─────────┘    └─────────────┘
```

Windows are assigned to slots by name. When layout changes, windows remap to available slots (e.g., "left" in two-column, "left-top" in three-slot could both receive FileList).

## Default UI

```
┌─────────────────────┬────────────────────────────────────┐
│ Changed Files       │ Diff Preview                       │
├─────────────────────┤                                    │
│ > src/main.go    M  │ @@ -10,6 +10,8 @@                  │
│   src/ui.go      M  │  func main() {                     │
│   README.md      A  │ -    oldLine()                     │
│   go.mod         M  │ +    newLine()                     │
│                     │ +    anotherLine()                 │
│                     │  }                                 │
│                     │                                    │
├─────────────────────┴────────────────────────────────────┤
│ feature/blocks  [working]  4 files  +127 -43             │
└──────────────────────────────────────────────────────────┘
```

## Status Bar

Shows at-a-glance context:

```
┌──────────────────────────────────────────────────────────┐
│ feature/blocks  [working]  4 files  +127 -43             │
└──────────────────────────────────────────────────────────┘
  │                │          │        │
  │                │          │        └── Total diff stats
  │                │          └── File count
  │                └── Current diff mode
  └── Current branch
```

Optional elements (shown when relevant):
- `↑2 ↓0` - ahead/behind remote
- `[no changes]` - when working tree is clean

## FileList Window

### Display Modes

**Flat list** (default, MVP)
```
src/main.go           M
src/app.go            M
internal/git/git.go   A
README.md             M
```
- Full relative path
- Sorted alphabetically by path
- Simple j/k navigation

**Tree view** (future iteration)
```
▼ src/
    main.go           M
    app.go            M
▼ internal/
  ▼ git/
      git.go          A
README.md             M
```
- Collapsible directories
- `Enter` or `l` to expand, `h` to collapse
- Remember expanded state per session

Toggle between modes with keybinding (e.g., `t`).

### Content by Diff Mode

| Mode | Shows |
|------|-------|
| Working | Files with uncommitted changes (`git status`) |
| Branch | All files changed vs base branch (`git diff --name-status main...HEAD`) |

### Status Indicators
- `M` - Modified
- `A` - Added
- `D` - Deleted
- `?` - Untracked (working mode only)
- `R` - Renamed

## DiffView Window

### Display Format

**Unified diff** (MVP)
```
@@ -10,6 +10,8 @@ func main()
 context line
-removed line
+added line
+another new line
 context line
```
- Standard git diff output
- Colorized: green for `+`, red for `-`, cyan for `@@`
- No transformation needed, display git output directly

**Side-by-side** (future, maybe)
```
│ old                │ new                │
│ removed line       │                    │
│                    │ added line         │
```
- Requires parsing and alignment
- Needs more terminal width

### Scrolling

Uses `bubbles/viewport`:
- `j`/`k` or arrows: scroll line by line
- `Ctrl+d`/`Ctrl+u`: half-page scroll
- `g`/`G`: top/bottom
- Mouse wheel (if terminal supports)

### Content by Selection

- When file selected in FileList → show diff for that file
- When no selection → show combined diff for all files (or empty state)

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down in list / scroll diff |
| `k` / `↑` | Move up in list / scroll diff |
| `h` / `l` | Switch focused window |
| `Ctrl+d` | Scroll diff half-page down |
| `Ctrl+u` | Scroll diff half-page up |
| `g` | Go to top (diff) |
| `G` | Go to bottom (diff) |
| `Enter` | Select item |
| `Escape` | Close modal / unfocus |
| `q` | Quit |
| `r` | Refresh |
| `y` | Yank (copy) file path to clipboard |
| `o` | Open file in $EDITOR |
| `?` | Toggle help modal |
| `1` | Working diff mode |
| `2` | Branch diff mode (vs master) |
| `t` | Toggle tree/flat view (future) |
| `Tab` | Cycle through windows |

## Configuration

Config lives in code, not external files. Keeps things simple for now.

```go
// config/config.go

type Config struct {
    DefaultMode  string // "working" or "branch"
    BaseBranch   string // "main", "master", or auto-detect

    Layout       LayoutConfig
    Keys         KeyConfig
    Colors       ColorConfig
}

type LayoutConfig struct {
    DefaultRatio [2]int // left:right ratio, e.g. {30, 70}
}

type ColorConfig struct {
    Added          string
    Removed        string
    Context        string
    BorderFocused  string
    BorderUnfocused string
}

var Default = Config{
    DefaultMode: "working",
    BaseBranch:  "", // auto-detect main/master
    Layout: LayoutConfig{
        DefaultRatio: [2]int{30, 70},
    },
    Colors: ColorConfig{
        Added:           "#a6e3a1",
        Removed:         "#f38ba8",
        Context:         "#cdd6f4",
        BorderFocused:   "#89b4fa",
        BorderUnfocused: "#45475a",
    },
}
```

External config file can be added later if needed.

## CLI Arguments

```
blocks [flags] [path]

Arguments:
  path              Target directory (default: current dir)

Flags:
  -m, --mode        Start in mode: working, branch (default: working)
  -b, --base        Base branch for branch mode (default: auto-detect)
  -h, --help        Show help
  -v, --version     Show version
```

## Error States

| Condition | Behavior |
|-----------|----------|
| Not a git repo | Show message: "Not a git repository" with hint |
| No changes | Show empty state: "No changes" (mode-aware message) |
| Git command fails | Show error in status bar, keep last good state |
| Base branch not found | Fall back to HEAD~10 or show config hint |

## Similar Tools & Differentiation

| Tool | Focus | Gap |
|------|-------|-----|
| [diffnav](https://github.com/dlvhdr/diffnav) | Git diff with file tree | No branch diff mode, no commit list |
| [critique](https://github.com/remorses/critique) | AI change review | Requires Bun, focused on explanations |
| [tuicr](https://github.com/agavra/tuicr) | Human-in-loop AI review | Narrower scope |
| [GitUI](https://github.com/gitui-org/gitui) | Full git TUI | Built for git ops, not AI workflow |
| VS Code / Cursor | Traditional IDE | Built for human writing code |

**Blocks differentiators:**
- Branch diff mode (see all work vs master, not just uncommitted)
- Docs integration (specs alongside implementation)
- Read-first design (review, don't edit)
- AI-native workflow (companion, not replacement for agent)

## Technical Stack

- **Language**: Go
- **TUI Framework**: Bubbletea
- **Styling**: Lipgloss
- **Git Operations**: Shell out to git CLI (simpler than go-git for our needs)

## Project Structure

```
blocks/
├── main.go                 # Entry point only - parse flags, start app
├── cmd/
│   └── root.go             # CLI setup (cobra if needed later)
├── internal/
│   ├── app/
│   │   ├── app.go          # tea.Model, Init, top-level Update/View
│   │   ├── state.go        # Shared state struct & state transitions
│   │   └── messages.go     # All message types
│   ├── config/
│   │   └── config.go       # Config structs & defaults
│   ├── layout/
│   │   ├── layout.go       # Layout tree & rendering
│   │   ├── node.go         # LayoutNode types
│   │   └── presets.go      # Built-in layout presets
│   ├── window/
│   │   ├── window.go       # Window interface
│   │   ├── base.go         # Common window functionality
│   │   ├── filelist.go     # FileList implementation
│   │   ├── diffview.go     # DiffView implementation
│   │   ├── commitlist.go   # CommitList implementation
│   │   └── help.go         # Help modal implementation
│   ├── git/
│   │   ├── git.go          # Git interface
│   │   ├── status.go       # File status operations
│   │   ├── diff.go         # Diff operations
│   │   └── log.go          # Commit log operations
│   ├── ui/
│   │   ├── styles.go       # Lipgloss styles
│   │   ├── colors.go       # Color definitions
│   │   └── borders.go      # Border helpers
│   └── keys/
│       └── keys.go         # Key definitions & help text
├── go.mod
├── go.sum
└── docs/
    └── design.md
```

## Code Architecture

### Principles

1. **Single responsibility** - Each package does one thing
2. **Dependency injection** - Pass dependencies, don't import globals
3. **Interface boundaries** - Packages communicate via interfaces
4. **Testable** - Business logic separated from TUI rendering

### Package Dependencies

```
main
  └── app
        ├── config
        ├── layout
        │     └── window (interface only)
        ├── window
        │     ├── git
        │     └── ui
        ├── git
        ├── ui
        └── keys
```

### Key Interfaces

```go
// window/window.go
type Window interface {
    // Update handles input when focused
    Update(msg tea.Msg) (Window, tea.Cmd)

    // View renders the window content
    View(width, height int) string

    // Focus state
    Focused() bool
    SetFocus(bool)

    // Identity
    Name() string
}

// git/git.go
type GitClient interface {
    // Status returns changed files
    Status() ([]FileStatus, error)

    // Diff returns diff for a file (or all files if empty)
    Diff(file string, mode DiffMode) (string, error)

    // Log returns commits on current branch vs base
    Log(base string) ([]Commit, error)

    // BaseBranch detects main/master
    BaseBranch() (string, error)
}

// layout/layout.go
type Layout interface {
    // Render renders all windows in the layout
    Render(width, height int, windows map[string]Window) string

    // FocusNext moves focus to next window
    FocusNext()

    // FocusPrev moves focus to previous window
    FocusPrev()

    // FocusedWindow returns currently focused window name
    FocusedWindow() string
}
```

### State Management

State is centralized in App. Features communicate via messages.

```go
// state.go - shared state
type State struct {
    // Selection
    SelectedFile string
    DiffMode     DiffMode

    // Data
    Files   []FileStatus
    Diff    string
    Commits []Commit

    // UI
    FocusedWindow string
    ActiveModal   string  // empty if no modal
}

// State transitions
func (s *State) SelectFile(path string) {
    s.SelectedFile = path
}

func (s *State) SetDiffMode(mode DiffMode) {
    s.DiffMode = mode
    s.SelectedFile = ""  // reset selection on mode change
}
```

```go
// messages.go - all message types
type FileSelectedMsg struct{ Path string }
type DiffModeChangedMsg struct{ Mode DiffMode }
type FilesLoadedMsg struct{ Files []FileStatus }
type DiffLoadedMsg struct{ Content string }
type CommitsLoadedMsg struct{ Commits []Commit }
type ErrorMsg struct{ Err error }
```

### Message Flow

```
User Input
    │
    ▼
App.Update()
    │
    ├── Global keys (quit, mode switch, modal toggle)
    │
    └── Delegate to focused feature
            │
            ▼
        Feature.Update()
            │
            └── Returns command (emits message)
                    │
                    ▼
                App.Update() receives message
                    │
                    └── Updates State, triggers re-render
```

## Development Phases

### Phase 1: Foundation
- [ ] Initialize Go module with dependencies
- [ ] Set up project structure (internal/, cmd/)
- [ ] Define core interfaces (Window, GitClient, Layout)
- [ ] Config package with defaults
- [ ] Basic app skeleton with tea.Model

### Phase 2: UI Framework
- [ ] Styles package (colors, borders)
- [ ] Keys package (keybindings)
- [ ] Base window implementation
- [ ] Layout node types
- [ ] Horizontal split rendering

### Phase 3: Windows (with mock data)
- [ ] FileList window with j/k navigation
- [ ] DiffView window with scrolling
- [ ] Focus management between windows
- [ ] Help modal (floating)

### Phase 4: Git Integration
- [ ] GitClient implementation (shell out to git)
- [ ] Status parsing
- [ ] Diff parsing with highlighting
- [ ] Wire windows to git client
- [ ] Diff mode switching (working/branch)

### Phase 5: CommitList & Polish
- [ ] CommitList window
- [ ] Log parsing
- [ ] Layout presets
- [ ] Responsive collapse behavior
- [ ] Error states (no repo, no changes)


## Decisions

### Large Diffs
Truncate at 10,000 lines. Show hint: `[truncated - showing first 10,000 lines]`

Binary files display git's default message: `Binary files a/foo.png and b/foo.png differ`

### Base Branch Detection
Hybrid approach:
1. Try `git config init.defaultBranch`
2. Try common names: `main`, `master`
3. Try remotes: `origin/main`, `origin/master`
4. If all fail: show error with hint to use `--base` flag

### CommitList Content
Show branch commits only: `git log <base>..HEAD`

When on base branch (e.g., working directly on main):
- Compare against remote: `git log origin/main..HEAD` (unpushed commits)
- If no remote or nothing unpushed: show empty state "No commits ahead"

## To Explore: Docs Integration

The workflow: write markdown specs → AI implements → review changes.

Docs are first-class in this workflow. Potential features:

| Idea | Description |
|------|-------------|
| **DocsList** | Window showing project docs (*.md files) |
| **DocsView** | Markdown viewer/preview in terminal |
| **Spec linking** | Associate a spec with current work/branch |
| **Split context** | Spec on left, implementation diff on right |
| **Doc changes** | Highlight when AI modifies docs vs code |

Questions to answer:
- Where do docs live? (`docs/`, root, anywhere?)
- How to identify "spec" docs vs other markdown?
- Render markdown or show raw?
- How does this connect to the diff workflow?
