# TimeCop Architecture

## Overview

TimeCop is a terminal UI for code review built with Rust and Ratatui.

## Component Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                                   main.rs                                        │
│  (src/main.rs)                                                                  │
│                                                                                  │
│  Responsibilities:                                                               │
│  ├── Terminal setup (raw mode, alternate screen)                                │
│  ├── Logger initialization (env_logger)                                         │
│  ├── Create App and EventHandler                                                │
│  └── Run main event loop                                                        │
└─────────────────────────────────────────────────────────────────────────────────┘
         │                              │
         │ creates                      │ creates
         ▼                              ▼
┌─────────────────────────┐    ┌─────────────────────────────────────────────────┐
│      EventHandler       │    │                      App                         │
│  (src/event.rs)         │    │  (src/app.rs)                                   │
│                         │    │                                                  │
│  State:                 │    │  State:                                          │
│  ├── rx: Receiver       │    │  ├── mode: AppMode                               │
│  ├── paused: AtomicBool │    │  ├── focused: FocusedWindow                      │
│  └── watcher: Debouncer │    │  ├── files: Vec<StatusEntry>                     │
│                         │    │  ├── selected_pr: Option<PrInfo>                 │
│  Methods:               │    │  ├── async_loader: AsyncLoader                   │
│  ├── next() → AppEvent  │    │  └── *_state: widget states                      │
│  ├── pause()            │    │                                                  │
│  └── resume()           │    │  Methods:                                        │
│                         │    │  ├── handle_key() → updates state                │
└─────────────────────────┘    │  ├── handle_tick() → poll async loaders          │
         │                     │  ├── refresh() → reload git data                 │
         │ sends               │  └── render() → draw UI                          │
         ▼                     └─────────────────────────────────────────────────┘
    AppEvent                            │
    ├── Key(KeyEvent)                   │ uses
    ├── Tick                            ▼
    ├── FileChanged          ┌─────────────────────────────────────────────────────┐
    └── Resize               │                    Modules                          │
                             │                                                     │
                             │  ┌─────────────────┐    ┌─────────────────────────┐ │
                             │  │   GitClient     │    │    GitHubClient         │ │
                             │  │ (src/git/)      │    │  (src/github/)          │ │
                             │  │                 │    │                         │ │
                             │  │ • status()      │    │ • list_open_prs()       │ │
                             │  │ • diff()        │    │ • get_pr_by_number()    │ │
                             │  │ • log()         │    │ • approve_pr()          │ │
                             │  │ • read_file()   │    │ • request_changes()     │ │
                             │  │                 │    │ • comment_pr()          │ │
                             │  │ Uses: libgit2   │    │ • add_line_comment()    │ │
                             │  │                 │    │  Uses: gh CLI           │ │
                             │  └─────────────────┘    └─────────────────────────┘ │
                             │                                                     │
                             │  ┌─────────────────────────────────────────────────┐│
                             │  │   AsyncLoader (src/async_loader.rs)             ││
                             │  │                                                 ││
                             │  │   Manages background loading:                   ││
                             │  │   ├── load_stats() → DiffStats                  ││
                             │  │   ├── load_pr_list() → Vec<PrSummary>           ││
                             │  │   ├── load_pr_details() → PrInfo                ││
                             │  │   └── poll_*() → check for completed loads      ││
                             │  └─────────────────────────────────────────────────┘│
                             └─────────────────────────────────────────────────────┘
```

## Event Flow

```
┌──────────┐    ┌──────────────┐    ┌────────────────┐    ┌─────────────┐
│  User    │    │ EventHandler │    │      App       │    │  Terminal   │
└────┬─────┘    └──────┬───────┘    └───────┬────────┘    └──────┬──────┘
     │                 │                    │                    │
     │ keypress        │                    │                    │
     │────────────────>│                    │                    │
     │                 │                    │                    │
     │                 │ AppEvent::Key      │                    │
     │                 │───────────────────>│                    │
     │                 │                    │                    │
     │                 │                    │ handle_key()       │
     │                 │                    │ ─ check modals     │
     │                 │                    │ ─ check global keys│
     │                 │                    │ ─ delegate to      │
     │                 │                    │   focused window   │
     │                 │                    │ ─ update state     │
     │                 │                    │                    │
     │                 │                    │ render()           │
     │                 │                    │───────────────────>│
     │                 │                    │                    │
     │  UI updated     │                    │                    │
     │<───────────────────────────────────────────────────────────
     │                 │                    │                    │
```

## Async Data Loading

```
┌─────────────────┐                      ┌─────────────────┐
│                 │   AsyncLoader        │                 │
│   Main Thread   │   .load_stats()      │ Background      │
│   (App)         │ ──────────────────►  │ Thread          │
│                 │                      │                 │
│                 │ ◄──────────────────  │ • GitClient     │
│                 │   poll_stats()       │ • diff_stats()  │
└─────────────────┘                      └─────────────────┘

┌─────────────────┐                      ┌─────────────────┐
│                 │   AsyncLoader        │                 │
│   Main Thread   │   .load_pr_list()    │ Background      │
│   (App)         │ ──────────────────►  │ Thread          │
│                 │                      │                 │
│                 │ ◄──────────────────  │ • GitHubClient  │
│                 │   poll_pr_list()     │ • gh CLI        │
└─────────────────┘                      └─────────────────┘

┌─────────────────┐                      ┌─────────────────┐
│                 │   AsyncLoader        │                 │
│   Main Thread   │   .load_pr_details() │ Background      │
│   (App)         │ ──────────────────►  │ Thread          │
│                 │                      │                 │
│                 │ ◄──────────────────  │ • PR details    │
│                 │   poll_pr_details()  │ • Reviews       │
└─────────────────┘                      │ • Comments      │
                                         └─────────────────┘

On each Tick:
  1. poll_stats() → apply DiffStats if ready
  2. poll_pr_list() → apply PR list if ready
  3. poll_pr_details() → apply PrInfo if ready
  4. Trigger new loaders if needed (e.g., PR poll interval)
```

## UI Widget Hierarchy

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Terminal Frame                                      │
│                                                                                  │
│  ┌─────────────────────────┐  ┌─────────────────────────────────────────────┐   │
│  │                         │  │                                             │   │
│  │      FileList           │  │              DiffView                       │   │
│  │  (file_list.rs)         │  │          (diff_view.rs)                     │   │
│  │                         │  │                                             │   │
│  │  • Tree view of files   │  │  PreviewContent:                            │   │
│  │  • Directory collapse   │  │  ├── FileDiff (side-by-side)                │   │
│  │  • Status indicators    │  │  ├── FolderDiff (combined)                  │   │
│  │                         │  │  ├── FileContent (browse mode)              │   │
│  ├─────────────────────────┤  │  ├── PrDetails (PR info view)               │   │
│  │                         │  │  ├── Loading (loading state)                │   │
│  │     PrListPanel         │  │  └── Empty                                  │   │
│  │  (pr_info.rs)           │  │                                             │   │
│  │                         │  │  Syntax highlighting via Syntect            │   │
│  │  • Open PRs list        │  │                                             │   │
│  │  • 2-line per PR format │  │                                             │   │
│  │  • Review indicators    │  │                                             │   │
│  │  • Current branch mark  │  │                                             │   │
│  └─────────────────────────┘  └─────────────────────────────────────────────┘   │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                              Status Bar                                      ││
│  │  branch | mode | file count | +added -removed                               ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                           HelpModal (overlay)                                ││
│  │                         (help.rs, toggled with ?)                           ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────────┐│
│  │                          InputModal (overlay)                                ││
│  │                    (input_modal.rs, for PR actions)                         ││
│  │                                                                              ││
│  │  ReviewAction:                                                               ││
│  │  ├── Approve (confirmation: y/n)                                            ││
│  │  ├── RequestChanges (text input)                                            ││
│  │  ├── Comment (text input)                                                   ││
│  │  └── LineComment (text input with file:line context)                        ││
│  └─────────────────────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Module Structure

```
src/
├── main.rs              # Entry point, terminal setup, event loop
├── app.rs               # Main application state and logic (~700 lines)
├── async_loader.rs      # Background data loading management
├── event.rs             # Event handling (keyboard, file watching, ticks)
├── config.rs            # Configuration, colors, layout settings
├── git/
│   ├── mod.rs           # Git module exports
│   ├── types.rs         # Git data structures (FileStatus, AppMode, StatusEntry)
│   └── client.rs        # Git operations using libgit2
├── github/
│   └── mod.rs           # GitHub API client using gh CLI (~500 lines)
│                        # Includes PR listing, details, and review actions
└── ui/
    ├── mod.rs           # UI module exports
    ├── layout.rs        # Layout computation (responsive grid)
    ├── syntax.rs        # Syntax highlighting with Syntect
    └── widgets/
        ├── mod.rs
        ├── diff_parser.rs   # Diff parsing utilities (extracted from diff_view)
        ├── diff_view.rs     # Diff/content preview widget
        ├── file_list.rs     # Tree view widget
        ├── help.rs          # Help modal widget
        ├── input_modal.rs   # Review action input modal
        └── pr_info.rs       # PR list panel widget
```

## File Watcher Flow

```
┌─────────────────┐
│   Repository    │
│   (recursive)   │
│   + .gitignore  │
└────────┬────────┘
         │
         │ notify crate watches
         ▼
┌─────────────────┐
│   Debouncer     │
│   (300ms)       │
└────────┬────────┘
         │
         │ DebouncedEventKind::Any
         │ (filtered by .gitignore)
         ▼
┌─────────────────┐
│  EventHandler   │
│  tx.send(       │
│    FileChanged) │
└────────┬────────┘
         │
         │ rx.recv()
         ▼
┌─────────────────┐
│      App        │
│  refresh()      │
│  ─ reload files │
│  ─ reload diff  │
│  ─ update UI    │
└─────────────────┘
```

## Application Modes

```
                    ┌─────────────────┐
                    │                 │
         ┌─────────>│    Changes      │<─────────┐
         │          │   (Mode 1)      │          │
         │          │ All changes vs  │          │
         │          │ base branch     │          │
         │          │ ● = uncommitted │          │
         │          └────────┬────────┘          │
         │                   │                   │
    press 1             press m              press 1
         │                   │                   │
         │                   ▼                   │
┌────────┴────────┐                     ┌───────┴────────┐
│                 │                     │                │
│     Docs        │◄───────────────────►│    Browse      │
│   (Mode 3)      │      press m        │   (Mode 2)     │
│                 │                     │                │
│  *.md files     │                     │  All files     │
│                 │                     │                │
└─────────────────┘                     └────────────────┘
```

## Key Input Handling

```
┌──────────────────────────┐
│ KeyEvent received        │
└────────────┬─────────────┘
             │
             ▼
    ╔════════════════════╗
   ╱  InputModal visible? ╲
  ╱                       ╲
 yes                      no
  │                        │
  ▼                        ▼
┌────────────────┐   ╔═══════════════╗
│ Handle modal:  │  ╱ show_help?     ╲
│ • y/n confirm  │ ╱                  ╲
│ • text input   │yes                 no
│ • Enter submit │  │                  │
│ • Esc cancel   │  ▼                  ▼
└────────────────┘ ┌────────────┐  ╔════════════════╗
                   │ Only handle│ ╱ Global key?     ╲
                   │ ? or Esc   │╱  (q, ?, r, Tab,   ╲
                   │ to close   │╱   m, 1-4, y, o)   ╲
                   └────────────┘yes                  no
                                 │                    │
                                 ▼                    ▼
                        ┌──────────────┐    ┌──────────────────┐
                        │ Handle       │    │ Delegate to      │
                        │ globally:    │    │ focused window:  │
                        │ • quit       │    │                  │
                        │ • mode switch│    │ • FileList keys  │
                        │ • yank/open  │    │ • PrList keys    │
                        │ • refresh    │    │   (a, x, c)      │
                        └──────────────┘    │ • Preview keys   │
                                            └──────────────────┘
```

## PR Review Flow

```
┌──────────┐    ┌──────────────┐    ┌─────────────────┐    ┌──────────────┐
│  User    │    │   PrList     │    │   InputModal    │    │  GitHub CLI  │
└────┬─────┘    │   focused    │    └────────┬────────┘    └──────┬───────┘
     │          └──────┬───────┘             │                    │
     │                 │                     │                    │
     │ press 'a'       │                     │                    │
     │────────────────>│                     │                    │
     │                 │                     │                    │
     │                 │ show Approve modal  │                    │
     │                 │────────────────────>│                    │
     │                 │                     │                    │
     │ press 'y'       │                     │                    │
     │────────────────>│────────────────────>│                    │
     │                 │                     │                    │
     │                 │                     │ gh pr review       │
     │                 │                     │ --approve          │
     │                 │                     │───────────────────>│
     │                 │                     │                    │
     │                 │                     │   success/error    │
     │                 │                     │<───────────────────│
     │                 │                     │                    │
     │                 │ hide modal          │                    │
     │                 │<────────────────────│                    │
     │                 │                     │                    │
     │                 │ reload PR details   │                    │
     │                 │───────────────────────────────────────-->│
     │                 │                     │                    │
```

## External Editor Integration

```
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│ User    │────>│ App     │────>│ Command │────>│ Editor  │
│ press o │     │ queues  │     │ execute │     │ opens   │
└─────────┘     │ command │     └─────────┘     └─────────┘
                └─────────┘
                     │
                     ▼
            ┌─────────────────┐
            │ Terminal state: │
            │ 1. Pause events │
            │ 2. Leave alt    │
            │    screen       │
            │ 3. Disable raw  │
            │    mode         │
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │ Run $EDITOR     │
            │ with path:line  │
            │                 │
            │ vim +42 file.rs │
            │ hx file.rs:42   │
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────┐
            │ Restore:        │
            │ 1. Enable raw   │
            │ 2. Enter alt    │
            │    screen       │
            │ 3. Resume events│
            │ 4. Refresh data │
            └─────────────────┘
```

## Logging

Application logging is configured via the `RUST_LOG` environment variable:

```bash
# Default: warnings only
timecop

# Enable debug logging
RUST_LOG=debug timecop

# Enable trace logging for async_loader
RUST_LOG=timecop::async_loader=trace timecop
```

Log messages are written to stderr and include:
- Background task failures (PR loading, stats loading)
- GitHub CLI errors
- Git operation errors
