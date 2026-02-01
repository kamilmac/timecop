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

## Architecture

Kimchi is a TUI app built with Rust/Ratatui. It uses a centralized App struct that manages state and delegates to widget components.

### Core Concepts

- **App** (`src/app.rs`): Main application state and event handling. Manages focus, mode, and coordinates between widgets.
- **Widgets** (`src/ui/widgets/`): Stateful UI components (FileList, CommitList, DiffView, HelpModal). Each implements Ratatui's `StatefulWidget`.
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
- `src/config.rs` - Colors, layout config, timing
- `src/git/client.rs` - GitClient with libgit2
- `src/git/types.rs` - FileStatus, DiffMode, AppMode enums
- `src/ui/widgets/file_list.rs` - Tree view with directory structure
- `src/ui/widgets/diff_view.rs` - Side-by-side diff with inline comments

### Modes

Press `m` to cycle through modes, or use number keys:
- `1` - changed:working (uncommitted changes)
- `2` - changed:branch (all changes vs base)
- `3` - browse (all tracked files)
- `4` - docs (markdown files only)

### Adding a Widget

1. Create `src/ui/widgets/newwidget.rs` implementing `StatefulWidget`
2. Add state struct with methods for navigation/updates
3. Export from `src/ui/widgets/mod.rs`
4. Add to App struct and render in `App.render()`

### Key Bindings

Defined in `src/event.rs` as `KeyInput` helper methods. Add new bindings there and handle in `App.handle_key()`.
