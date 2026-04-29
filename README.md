# TimeCop

<img src="screenshots/timecop.png" width="700" />

> "Navigate commits like Van Damme navigates time. But sitting down."

A terminal UI for reviewing GitHub PRs and local branches — built for the agent loop. Open a PR and edit it like you would on github.com. Open a branch and yank code straight to your clipboard for your coding agent.

> Press `?` inside the app to see all keybindings.

## Two modes, picked from the argument shape

```bash
timecop              # current branch (full diff vs origin/main, includes workdir)
timecop <branch>     # named branch (committed only)
timecop <PR#>        # open the PR
```

### PR mode

Edit like github.com — every action posts immediately, no draft staging:

- `c` — comment / reply (line → new comment, thread → reply)
- `R` — toggle resolved on a thread
- `+` — 👍 react on a comment
- `Enter` — submit verdict (approve / request changes)

### Branch mode

Read-only diff with notes you can take to your agent:

- `,` — toggle full diff ↔ uncommitted-only
- `c` — leave a draft note (current branch only)
- `Y` — yank all drafts to clipboard, with code excerpts and the branch name
- `y` — yank just what's under cursor
- `o` — open the file in `$EDITOR` at the right line

Clipboard format:

```
Branch: feat/foo (vs refs/remotes/origin/main)

src/app.rs:142  (in fn handle_key)
> let x = unwrap();

should return Result
```

## Install

```bash
git clone https://github.com/kamilmac/timecop
cd timecop
cargo install --path .
```

Requires `git` and the [`gh` CLI](https://cli.github.com/) (for PR mode — `gh auth login`).

## License

MIT
