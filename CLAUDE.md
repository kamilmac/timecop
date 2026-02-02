# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build              # Build debug binary
cargo build --release    # Build release binary
cargo run                # Run in current directory
cargo run -- /path       # Run in specific directory
```

No tests currently exist.

## Code Style Preferences

- **Separation by feature, not type** - Group related functionality together rather than separating by file type (e.g., keep handlers with their views)
- **Tight, focused code** - Remove dead code aggressively, avoid over-abstraction
- **Inline with existing patterns** - Follow conventions already established in the codebase

## Architecture

Kimchi is a TUI code review app built with Rust/Ratatui for reviewing GitHub PRs.

### Core Concepts

- **App** (`src/app.rs`): Main application state and event handling. Manages focus, timeline position, and coordinates between widgets.
- **Widgets** (`src/ui/widgets/`): Stateful UI components (FileList, DiffView, PrListPanel, HelpModal). Each implements Ratatui's `StatefulWidget`.
- **Git Client** (`src/git/`): Native git operations via libgit2 (git2 crate). No shell commands.
- **GitHub** (`src/github/`): PR info fetching via `gh` CLI.

### Event Flow

```
Terminal Event → EventHandler thread → App.handle_key()
    → Update widget states → App.render() → Ratatui draws
```

### Key Files

- `src/main.rs` - Entry point, terminal setup, main loop
- `src/app.rs` - App struct, event handling, state management
- `src/event.rs` - Event handler thread, key input helpers
- `src/config.rs` - Colors, timing config
- `src/git/client.rs` - GitClient with libgit2
- `src/git/types.rs` - FileStatus, TimelinePosition, StatusEntry
- `src/ui/widgets/file_list.rs` - Tree view with directory structure
- `src/ui/widgets/diff_view.rs` - Side-by-side diff with inline comments

### Timeline Navigation

Use `,` and `.` to navigate through PR history:
- **wip** - Only uncommitted changes (HEAD → working tree)
- **current** (◆) - Full diff against base branch
- **-1 to -6** - Individual commit diffs

The Files panel title shows commit context when viewing historical commits.

### Adding a Widget

1. Create `src/ui/widgets/newwidget.rs` implementing `StatefulWidget`
2. Add state struct with methods for navigation/updates
3. Export from `src/ui/widgets/mod.rs`
4. Add to App struct and render in `App.render()`

### Key Bindings

Defined in `src/event.rs` as `KeyInput` helper methods. Add new bindings there and handle in `App.handle_key()`.
