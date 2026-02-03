# Kimchi

Terminal UI for reviewing code changes. Built for the AI coding era.

<p>
  <img src="screenshots/diff-view.png" width="49%" />
  <img src="screenshots/pr-view.png" width="49%" />
</p>

## Why

Coding agents changed how we write code. They commit fast, iterate fast, produce more code than ever. **Reviews are the new bottleneck.**

Traditional tools don't fit this workflow:

1. **Working tree diff is useless.** Agents commit constantly. You need the full picture — diff against base branch.
2. **Context switching kills flow.** Browser for PRs, terminal for code, editor for fixes. Too much friction.
3. **Code review is the job now.** When agents write, you review. The diff is the artifact, not the source file.

Kimchi sits next to your coding agent. Watch changes happen in real-time, scroll through commit history, review PRs — all without leaving the terminal.

## Features

- **Real-time file watching** — See changes as your agent writes code
- **Timeline navigation** — Scroll through commit history with `,` and `.`
- **Split or unified diff** — Toggle with `s`, auto-switches based on terminal width
- **PR workflow** — List PRs, view details, approve/comment without browser
- **Syntax highlighting** — Full language support in diff views
- **Open in editor** — Jump to exact line with `o`

## Install

```bash
git clone https://github.com/kmacinski/kimchi
cd kimchi
cargo build --release
cp target/release/kimchi ~/.local/bin/
```

## Requirements

- **Git**
- **gh CLI** (optional) — for PR list, reviews, and comments. [Install here](https://cli.github.com/). Without it, PR features are disabled but local git operations work fine.

## Usage

```bash
kimchi              # current directory
kimchi /path/to/repo
```

Run it alongside your coding agent. Changes refresh automatically.

## Keys

| Key | Action |
|-----|--------|
| `j/k` | Navigate |
| `J/K` | Fast scroll (5 lines) |
| `g/G` | Jump to top/bottom |
| `h/l` | Collapse/expand folders |
| `Tab` | Cycle panes (Files → Preview → PRs) |
| `s` | Toggle split/unified diff |
| `,/.` | Timeline: older/newer commit |
| `o` | Open in $EDITOR at current line |
| `y` | Copy file path |
| `Enter` | Open diff / checkout PR branch |
| `r` | Refresh |
| `?` | Help |
| `q` | Quit |

**PR actions** (requires `gh`): `a` approve, `x` request changes, `c` comment

## Timeline

The TIMECOP header is your timeline. Navigate through branch history:

```
◆─◆─T─I─M─E─C─O─P─◆─◆
                  │ └── wip (uncommitted changes)
                  └──── all changes (base → HEAD) ← default
              └──────── -1 (latest commit)
          └──────────── -2
      └──────────────── -3 ... -9
```

Use `,` to go back in history, `.` to go forward. The selected position glows red. A label shows your current view: "wip", "all changes", or "-N".

**Use case:** Agent made 5 commits. Step through each one to understand the changes, then view "all changes" for the complete picture before approving.

## Workflow

1. Start your coding agent in one terminal
2. Run `kimchi` in another
3. Watch changes appear in real-time as agent works
4. Use timeline to review individual commits
5. Press `o` to open files in your editor for quick fixes
6. When done, press `a` to approve the PR

## License

MIT
