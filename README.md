# Kimchi

**AI writes code. You review it.**

```bash
go install github.com/kmacinski/blocks@latest
kimchi
```

---

## The Problem

You're using Claude, Cursor, Copilot, whatever. AI writes code. You run `git diff` a thousand times. You alt-tab between terminal and editor. You scroll through walls of green and red. You miss things. You approve anyway.

## The Solution

A terminal UI that shows you what changed. That's it.

```
┌─────────────────────┬──────────────────────────────────────┐
│ Files (4)           │ src/main.go ────────────────── 42%   │
│ ▼ src/              │  12 │ func main() {          │       │
│   > main.go      M  │  13 │-    oldLine()          │       │
│     app.go       M  │  14 │+    newLine()          │       │
│ ▼ internal/         │  15 │ }                      │       │
│       git.go     A  │                                      │
├─────────────────────┤                                      │
│ Commits (3)         │                                      │
│ > abc123 Add feat   │                                      │
├─────────────────────┴──────────────────────────────────────┤
│ feature/thing  [branch]  4 files  +127 -43                 │
└────────────────────────────────────────────────────────────┘
```

---

## What It Does

- Shows changed files in a tree
- Shows diffs side-by-side
- Shows commits on your branch
- Shows PR comments if you have `gh` installed
- Refreshes automatically when files change
- Vim keys because obviously

## What It Doesn't Do

- Edit files
- Run commands
- Integrate with your AI tool
- Solve world hunger

It's a viewer. Read-only. On purpose.

---

## Keys

| Key | What |
|-----|------|
| `j`/`k` | Up/down |
| `Tab` | Next pane |
| `m` | Cycle modes |
| `1-4` | Jump to mode |
| `y` | Copy path |
| `o` | Open in $EDITOR |
| `q` | Quit |
| `?` | Help |

## Modes

| Key | Mode | Shows |
|-----|------|-------|
| `1` | working | Uncommitted changes |
| `2` | branch | All changes vs base branch |
| `3` | browse | All files |
| `4` | docs | Markdown files |

---

## Requirements

- Go 1.21+
- Git
- A terminal
- Optional: `gh` CLI for PR stuff

## Build

```bash
git clone https://github.com/kmacinski/blocks
cd blocks
go build -o kimchi .
./kimchi
```

---

MIT License. Do whatever.
