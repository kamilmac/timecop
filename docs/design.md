# TimeCop Design Document

A terminal UI for code review, built with Rust and Ratatui.

## Vision

When AI writes code, you need:
- **Visibility** - see what changed
- **Navigation** - understand the codebase
- **Review** - approve changes with confidence
- **Context** - PR comments alongside code

This is read-heavy, not write-heavy. The human reviews, the AI writes.

## Timeline Navigation

TimeCop lets you time-travel through PR history. The header shows your position:

```
T─I─M─E─C─O─P─○─○─○─●─[full]─[files]
              -3-2-1 wip full  files
              ◄─────────────────────►
              older            newer
```

| Position | Description |
|----------|-------------|
| `-N` | Single commit diff (HEAD~N → HEAD~(N-1)) |
| `wip` | Uncommitted changes (HEAD → working tree) |
| `full` | All changes vs base branch (default) |
| `files` | Browse all repository files |

Navigate with `,` (older) and `.` (newer).

### Diff Calculation

Diffs are calculated relative to the **merge-base** with remote:

```
     origin/main
           │
     A─────B─────C─────D      ← remote main
           │
           └──E──F──G──H      ← your branch (HEAD)
              │
              merge-base (B)
```

- **full** = B → H (all changes since branching)
- **wip** = H → working directory
- **-1** = G → H (most recent commit)

Uses `simplify_first_parent()` to ignore merge commits from main.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         main.rs                              │
│              Terminal setup, event loop                      │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
┌─────────────────┐  ┌─────────────┐  ┌─────────────────┐
│   EventHandler  │  │     App     │  │    Terminal     │
│   (event.rs)    │  │  (app.rs)   │  │   (ratatui)     │
│                 │  │             │  │                 │
│ • Keyboard      │  │ • State     │  │ • Raw mode      │
│ • Mouse         │  │ • Logic     │  │ • Rendering     │
│ • File watcher  │  │ • Commands  │  │                 │
│ • Tick events   │  │             │  │                 │
└─────────────────┘  └─────────────┘  └─────────────────┘
                              │
         ┌────────────────────┼────────────────────┐
         ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│  AsyncLoader    │  │  GitHubClient   │  │   UI Widgets    │
│                 │  │   (gh CLI)      │  │                 │
│ • PR list       │  │ • PR info       │  │ • FileList      │
│ • PR details    │  │ • Comments      │  │ • DiffView      │
│                 │  │ • Reviews       │  │ • PrListPanel   │
└─────────────────┘  │ • Actions       │  │ • PrDetailsView │
         │           └─────────────────┘  │ • HelpModal     │
         ▼                                │ • InputModal    │
┌─────────────────┐                       └─────────────────┘
│   GitClient     │
│  (libgit2)      │
│                 │
│ • Status        │
│ • Diff          │
│ • Log           │
└─────────────────┘
```

## Event Flow

```
User Input → EventHandler → App.handle_key() → State Update → render()
     ▲                                              │
     │              Widget State Updates ◄──────────┘
     │                      │
     └──────────────────────┘
```

1. EventHandler runs in separate thread, sends events via mpsc channel
2. App receives events, updates state, delegates to focused widget
3. On each frame, App renders all widgets with current state
4. Commands (like open editor) are queued and executed after render

### Async Loading

Background tasks managed by `AsyncLoader`:

```
┌─────────────────────────────────────────────────────────────┐
│                       AsyncLoader                            │
├─────────────────────────────────────────────────────────────┤
│  load_pr_list()    ──► spawns thread ──► poll_pr_list()     │
│  load_pr_details() ──► spawns thread ──► poll_pr_details()  │
└─────────────────────────────────────────────────────────────┘
                              │
                    mpsc channels for results
                              │
                              ▼
                    App.handle_tick() polls for completion
```

## Layout

```
┌─────────────────┬──────────────────────────────────────────┐
│   Header: T─I─M─E─C─O─P─○─○─●─[full]─[files]    ? help     │
├─────────────────┼──────────────────────────────────────────┤
│                 │                                          │
│    FileList     │              DiffView                    │
│                 │                                          │
│  ▼ src/         │        (preview panel)                   │
│    > app.rs  M  │                                          │
│    > main.rs M  │   Side-by-side or unified diff           │
│                 │   with syntax highlighting               │
├─────────────────┤   and inline comments                    │
│                 │                                          │
│  PrListPanel    │                                          │
│                 │                                          │
│  #42 Fix bug    │                                          │
│  #38 Add feat   │                                          │
│                 │                                          │
├─────────────────┴──────────────────────────────────────────┤
│  main  +42 -15                        full diff (base→head)│
└────────────────────────────────────────────────────────────┘
```

## Widgets

### FileList

Tree view of files with directory nesting.

```
Changed (4)
▼ src/
  > main.rs           M
    app.rs            M
▼ internal/
  ▼ git/
      client.rs       A
  README.md           M
```

- `▼`/`▶` prefix for expanded/collapsed directories
- Status indicators: M (modified), A (added), D (deleted), R (renamed)
- `h` collapses, `l` expands
- Comment indicator when file has PR comments

### DiffView

Side-by-side or unified diff viewer with syntax highlighting.

**Split mode (default):**
```
  12 │ context line            │   12 │ context line
  13 │-removed line            │      │
     │                         │   13 │+added line
  14 │ context line            │   14 │ context line
```

**Unified mode (auto-switches on narrow terminals):**
```
  12   context line
  13 - removed line
  13 + added line
  14   context line
```

**Inline PR comments:**
```
  37 │ let result = process(); │   37 │ let result = process();
     │ 💬 reviewer
     │    This could be optimized
  38 │ return result;          │   38 │ return result;
```

Toggle with `s`. Auto-switches to unified below 100 columns.

### PrListPanel

Shows open PRs for the repository.

```
Open PRs (3)
> #42 Fix auth bug          alice    ✓
  #38 Add dark mode         bob
  #35 Refactor API          charlie
```

- Loads asynchronously via gh CLI
- Shows PR number, title, author, review status
- `Enter` to checkout, `o` to open in browser

### PrDetailsView

Shows when PR list is focused - displays PR metadata, body, reviews, and comments.

### HelpModal

Overlay showing all keybindings, toggled with `?`.

### InputModal

Text input for PR review actions (approve, request changes, comment).

## Key Bindings

### Global

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit |
| `?` | Toggle help |
| `r` | Refresh |
| `s` | Toggle split/unified diff |
| `Tab` | Next pane |
| `Shift+Tab` | Previous pane |
| `,` | Timeline: older |
| `.` | Timeline: newer |
| `y` | Yank path to clipboard |
| `o` | Open in editor (or PR in browser) |

### Navigation

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `J` / `K` | Fast move (5 lines) |
| `Ctrl+d` / `Ctrl+u` | Page down/up |
| `g` / `G` | Top / bottom |
| `h` | Collapse folder |
| `l` | Expand folder |

### PR Review

| Key | Action |
|-----|--------|
| `a` | Approve PR |
| `x` | Request changes |
| `c` | Add comment (PR-level or line-level) |

## Data Structures

### Core Types

```rust
pub enum TimelinePosition {
    CommitDiff(usize),  // Single commit: HEAD~N → HEAD~(N-1)
    Wip,                // Uncommitted: HEAD → workdir
    FullDiff,           // All changes: merge-base → HEAD
    Browse,             // All repository files
}

pub enum FileStatus {
    Modified, Added, Deleted, Renamed, Untracked, Unchanged
}

pub struct StatusEntry {
    pub path: String,
    pub status: FileStatus,
}
```

### GitHub Types

```rust
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub reviews: Vec<Review>,
    pub comments: Vec<Comment>,
    pub file_comments: HashMap<String, Vec<Comment>>,
}

pub struct PrSummary {
    pub number: u64,
    pub title: String,
    pub author: String,
    pub branch: String,
    pub review_decision: Option<String>,
}
```

## Git Integration

Uses libgit2 (git2 crate) for native performance:

- Repository opening with path resolution
- Status checking via index/workdir comparison
- Diff generation between commits/trees
- Commit history traversal with first-parent
- Base branch auto-detection (origin/main, origin/master, main, master)

## GitHub Integration

Uses gh CLI for GitHub API access:

- PR list fetching for repository
- PR details with reviews and comments
- Inline comments mapped to file paths and lines
- PR review submission (approve, request changes, comment)
- PR branch checkout
- Polling every 120 seconds for updates

## Configuration

Centralized in `config.rs`:

**Colors (Catppuccin Mocha):**
- Added: Green
- Removed: Red/Pink
- Modified: Peach
- Header: Blue
- Comments: Yellow on dark background

**Timing:**
- PR poll interval: 120 seconds
- File watcher debounce: 300ms

**Layout:**
- Left panel: 30%
- Right panel: 70%

## Project Structure

```
src/
├── main.rs           # Entry point, terminal setup, event loop
├── app.rs            # Main application state and logic
├── async_loader.rs   # Background task management
├── event.rs          # Event handling, key input helpers
├── config.rs         # Colors, timing, theme
├── theme.rs          # Light/dark theme detection
├── git/
│   ├── mod.rs
│   ├── types.rs      # TimelinePosition, FileStatus, StatusEntry
│   └── client.rs     # Git operations using libgit2
├── github/
│   └── mod.rs        # GitHub API client using gh CLI
└── ui/
    ├── mod.rs
    ├── layout.rs     # Responsive layout computation
    ├── syntax.rs     # Syntax highlighting (syntect)
    └── widgets/
        ├── mod.rs
        ├── file_list/    # Tree view widget
        ├── diff_view/    # Diff preview with parser
        ├── pr_list/      # PR list panel
        ├── pr_details/   # PR details view
        ├── help/         # Help modal
        └── input/        # Input modal for reviews
```

## Performance

- Native libgit2 (no shell overhead for git operations)
- Async loading for PR list and details
- Debounced file watching (300ms)
- Lazy PR polling (120s intervals)
- Offset-based viewport rendering
- Syntax highlight caching per file
- Release build: LTO, single codegen unit, stripped binary

## Error Handling

- Uses `anyhow::Result<T>` throughout
- Background task failures logged, don't crash app
- Graceful fallbacks:
  - Missing gh CLI: PR features disabled
  - Missing base branch: falls back to working status
  - Binary files: shows "Binary file" message

## External Editor

Opens files in `$EDITOR` with line number support:

- **vim/nvim**: `+{line}` argument
- **helix**: `{file}:{line}` format

Terminal suspended during editor session, auto-refresh on close.
