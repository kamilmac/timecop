# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

For full architecture details, see [docs/design.md](docs/design.md).

## Build & Run

```bash
cargo build              # Build debug binary
cargo build --release    # Build release binary
cargo run                # Run in current directory
cargo run -- /path       # Run in specific directory
```

```bash
cargo test               # Run all tests
```

## Code Style

- **Feature-based organization** - Group related functionality together (e.g., widgets in subdirectories with mod.rs, state, rendering)
- **Tight, focused code** - Remove dead code aggressively, avoid over-abstraction
- **Follow existing patterns** - Match conventions already in the codebase

## Architecture Overview

TimeCop is a TUI for code review built with Rust/Ratatui.

```
src/
├── main.rs           # Entry point, terminal setup, main loop
├── app.rs            # Central state, event handling, rendering coordination
├── event.rs          # EventHandler thread, KeyInput helpers
├── async_loader.rs   # Background PR loading
├── config.rs         # Colors, timing, layout config
├── git/
│   ├── client.rs     # Git operations (libgit2)
│   └── types.rs      # TimelinePosition, FileStatus, StatusEntry
├── github/
│   └── mod.rs        # PR fetching via gh CLI
└── ui/
    ├── layout.rs     # Responsive layout
    ├── syntax.rs     # Syntax highlighting (syntect)
    └── widgets/
        ├── file_list/    # Tree view
        ├── diff_view/    # Diff preview + parser
        ├── pr_list/      # PR list panel
        ├── pr_details/   # PR details view
        ├── help/         # Help modal
        └── input/        # Input modal for reviews
```

### Core Components

- **App** (`app.rs`): Central coordinator. Manages focus, timeline position, widget states.
- **Widgets** (`ui/widgets/`): Each widget has its own subdirectory with state and rendering.
- **GitClient** (`git/client.rs`): Native git via libgit2. No shell commands.
- **GitHubClient** (`github/mod.rs`): PR operations via `gh` CLI.

### Event Flow

```
Terminal Event → EventHandler → App.handle_key() → Widget.handle_key()
    → Action returned → App.dispatch() → State update → App.render()
```

## Timeline Navigation

Navigate PR history with `,` (older) and `.` (newer):

```
T─I─M─E─C─O─P─○─○─○─●─[full]─[files]
              -3-2-1 wip full  files
```

| Position | What it shows |
|----------|---------------|
| `-1` to `-16` | Single commit diffs |
| `wip` | Uncommitted changes only |
| `full` | All changes vs base branch (default) |
| `files` | Browse all repository files |

Implemented in `TimelinePosition` enum (`git/types.rs`) and `switch_timeline()` in `app.rs`.

## Adding a Widget

1. Create directory `src/ui/widgets/newwidget/`
2. Add `mod.rs` with widget struct implementing `StatefulWidget`
3. Add state struct with `handle_key()` returning `Action`
4. Export from `src/ui/widgets/mod.rs`
5. Add state to `App` struct, render in `App.render()`

## Key Bindings

Defined in `src/event.rs` as `KeyInput::is_*` methods. Handle in `App.handle_key()` for global keys, or in widget `handle_key()` for widget-specific keys.

Global: `q` quit, `?` help, `r` refresh, `s` toggle view, `Tab` cycle panes, `,`/`.` timeline
Navigation: `j`/`k` move, `J`/`K` fast, `g`/`G` top/bottom, `h`/`l` collapse/expand
PR Review: `a` approve, `x` request changes, `c` comment
