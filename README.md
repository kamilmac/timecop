# TimeCop

> "Navigate commits like Van Damme navigates time. But sitting down."

A terminal UI for reviewing GitHub PRs and local branches — built for the agent loop. Open a PR and edit it like you would on github.com. Open a local branch and yank code straight to your clipboard for your coding agent.

> Press `?` inside the app to see all keybindings.

## Modes

### PR mode

`timecop <PR#>`

Renders the PR diff with existing review threads inline. Edits hit GitHub immediately — no draft staging.

| Key | Action |
|-----|--------|
| `c` | Comment / reply (auto-detect: line → new comment, thread → reply) |
| `R` | Toggle resolved on the thread under cursor |
| `+` | 👍 react on the comment under cursor |
| `Enter` | Verdict picker — `a` approve, `x` request changes |
| `r` | Refresh from GitHub |
| `y` | Yank code line / thread to clipboard with context |

### Branch mode

`timecop` (current branch) or `timecop <branch>`

Read-only diff against `merge-base(HEAD, origin/main)`. For the current branch, the working tree is included so you see uncommitted edits too.

| Key | Action |
|-----|--------|
| `y` | Yank code line / thread under cursor (file:line + context, prefixed with branch name) |
| `o` | Open file in `$EDITOR` at the right line |
| `r` | Refresh (recompute diff) |

Clipboard payload format:

```
Branch: feat/foo (vs refs/remotes/origin/main)

src/app.rs:142  (in fn handle_key)
> let x = unwrap();
```

Paste at Claude / Cursor / whoever.

## Movement

| Key | Action |
|-----|--------|
| `j` / `k` | Line down / up |
| `J` / `K` | Fast (5 lines) |
| `Ctrl-f` / `Ctrl-b` | Full page |
| `g g` / `G` | Top / bottom |
| `Space` | Toggle file / thread under cursor |
| `h` / `l` | Collapse / expand |
| `z` | Toggle all files |

## Install

```bash
git clone https://github.com/kamilmac/timecop
cd timecop
cargo install --path .
```

## Requirements

- Git
- [`gh` CLI](https://cli.github.com/) for PR mode (`gh auth login`)

## Architecture

Four vertical-slice modules under `src/`:

- **`app/`** — central state, key dispatch, clipboard formatter
- **`diff/`** — pure diff types + acquisition (libgit2 for branch, unified-diff parsing for PR)
- **`session/`** — IO boundary: branch or PR session, owns all `gh` and libgit2 calls
- **`ui/`** — read-only render layer (stacked diff, scroll, fold, theme, syntect highlighting, modals)

See [docs/design.md](docs/design.md).

## License

MIT
