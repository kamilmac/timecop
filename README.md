# TimeCop

"Navigate commits like Van Damme navigates time. But sitting down."

A terminal UI for reviewing pull requests and navigating commit history.

<img src="screenshots/timecop.png" height="350" />

> Press `?` inside the app to see all keybindings.

## Features

- **Timeline scrubbing** — Step through commits, wip changes, full diff, or browse all files with `,` and `.`
- **Side-by-side diffs** — Split or unified view with auto-switching on narrow terminals
- **Syntax highlighting** — Language-aware coloring for diffs and file content
- **Inline PR comments** — See review comments right in the diff where they belong
- **PR actions** — Comment, approve, or request changes without leaving the terminal
- **All PRs in one view** — Browse open pull requests, see review status, check out branches
- **Keyboard-driven** — Fast vim-style navigation, no mouse required

## Screenshot

<img src="screenshots/overview.png" width="700" />

## Install

**Quick install (macOS/Linux):**
```bash
curl -fsSL https://raw.githubusercontent.com/kamilmac/timecop/main/install.sh | sh
```

**Build from source:**
```bash
git clone https://github.com/kamilmac/timecop
cd timecop
cargo build --release
cp target/release/timecop ~/.local/bin/  # or anywhere in your PATH
```

## Requirements

- Git
- [gh CLI](https://cli.github.com/) — for PR features (list, approve, comment)
  - Run `gh auth login` to authenticate

## Usage

```bash
timecop              # Run in current directory
timecop /path/to/repo
```

### Key Bindings

| Key | Action |
|-----|--------|
| `,` `.` | Timeline: older / newer (commits → wip → full → files) |
| `j` `k` | Navigate up/down |
| `J` `K` | Fast navigate (5 lines) |
| `h` `l` | Collapse / expand folder |
| `Tab` | Cycle through panes |
| `s` | Toggle split/unified diff view |
| `o` | Open file in $EDITOR |
| `y` | Yank path to clipboard |
| `r` | Refresh |
| `c` | Add comment |
| `a` | Approve PR |
| `x` | Request changes |
| `?` | Show all keybindings |

## License

MIT
