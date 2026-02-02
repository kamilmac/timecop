# Kimchi

Terminal UI for reviewing code changes. Built for the AI coding era.

## Why

AI agents commit faster than you can review. This creates problems:

1. **Working tree diff is useless.** Agents commit often. You need diff against base branch — the full picture.
2. **PRs need faster access.** More code, more reviews. Browser switching kills flow.
3. **Code is secondary.** When agents write, you review. The diff is the artifact, not the source file.

Kimchi sits next to your AI agent. Shows changes, refreshes automatically, PRs are a keystroke away.

```
┌─ Files (4) ────────┬─ src/app.rs ─────────────────────────────┐
│ ▼ src/             │  12 │ fn main() {         fn main() {   │
│   > app.rs      M  │  13 │-    old_call();                    │
│     config.rs   M  │  14 │+                    new_call();    │
├────────────────────┤  15 │ }                   }              │
│                    │                                          │
├─ PRs (2) ──────────┴──────────────────────────────────────────┤
│●  #42 │ you        │ Add feature                     │  today │
│ ◆ #38 │ teammate   │ Fix bug                         │     2d │
└───────────────────────────────────────────────────────────────┘
```

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

## Keys

| Key | Action |
|-----|--------|
| `j/k` | Navigate |
| `J/K` | Fast scroll |
| `h/l` | Scroll diff horizontally |
| `g/G` | Top/bottom |
| `Tab` | Switch panes |
| `1-4` | Switch mode |
| `y` | Copy path |
| `o` | Open in $EDITOR |
| `?` | Help |
| `q` | Quit |

**PR actions** (requires `gh`): `a` approve, `x` request changes, `c` comment

## Modes

| `1` working | Uncommitted changes |
|-------------|---------------------|
| `2` branch | Changes vs base branch |
| `3` browse | All tracked files |
| `4` docs | Markdown files |

## License

MIT
