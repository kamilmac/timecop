# Kimchi

Terminal UI for reviewing code changes. Built for the AI coding era.

```
┌─ Files (4) ────────┬─ src/app.rs ─────────────────────────────┐
│ ▼ src/             │  12 │ fn main() {         fn main() {   │
│   > app.rs      M  │  13 │-    old_call();                    │
│     config.rs   M  │  14 │+                    new_call();    │
├─ Commits (3) ──────┤  15 │ }                   }              │
│ > abc123 Add feat  │                                          │
├─ PRs (2) ──────────┤                                          │
│ ● #42 @you         │                                          │
└────────────────────┴──────────────────────────────────────────┘
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
- **gh CLI** (optional) — for PR list, reviews, and comments. [Install here](https://cli.github.com/). Without it, the app works fine for local git operations; PR features are simply disabled.

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
