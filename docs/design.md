# TimeCop — Design

## What this is

A terminal tool for reviewing branch or PR changes the way you'd review a teammate's PR — except the teammate is a coding agent.

You open the tool on a branch (or PR), read the diff, leave inline comments, and either copy them out for an agent to act on, or apply them to a GitHub PR as a real review.

## The loop it serves

1. Agent writes code on a branch.
2. You open `timecop` on that branch (or `timecop <PR#>` if there's a PR).
3. You scroll the diff, leave comments and replies. Comments stay local.
4. You finish the review with one explicit action:
   - **Branch mode** → `y` copies all comments + surrounding code to the clipboard. Paste into your agent.
   - **PR mode** → `Enter` opens a verdict picker (approve / request changes / comment), submits one batched review to GitHub.
5. Agent addresses the comments. You re-open and review again.

The tool has no opinion about what happens between sessions. State lives where it belongs: drafts in memory, applied comments on GitHub.

## Invocation

```
timecop                  # current branch vs origin/main (merge-base)
timecop <branch>         # named branch vs origin/main (merge-base)
timecop <PR#>            # PR head vs PR base, via gh
```

A branch with no associated PR works in branch mode. A PR# works in PR mode. The tool picks the mode from the argument shape — no flag.

## The view

One window, top-down scroll. No tree pane, no split pane.

```
PR #142 · feat/foo → main · 8 files · 2 drafts                      142/890
─────────────────────────────────────────────────────────────────────────────
Description
  Adds the new event router. See INC-411.

▸ src/app.rs                         [+12 -4]   1 thread
▸ src/foo.rs                         [+3  -1]
▾ src/bar.rs                         [+8  -2]
   @@ fn parse @@
     pub fn parse(input: &str) -> Foo {
   -     input.trim().to_lowercase()
   +     input.trim_start().to_lowercase()
       ┃ alice · 2d
       ┃   does this still strip trailing whitespace?
       ┃   └─ [draft] you: yes — trim_start only changes leading
   }
   ...
▸ src/baz.rs                         [+1  -0]
```

- Files default to **all collapsed** — file headers form a table of contents you walk through.
- Unified diff only. No side-by-side.
- Threads render inline, anchored to their line. Drafts render with a `[draft]` marker.
- Resolved threads collapse to a one-liner. Outdated threads show with a marker but stay visible.

## The two modes

PR mode is branch mode plus an overlay. The data and view layer don't fork — only the source and sink differ.

|                | Branch                                  | PR                                                          |
| -------------- | --------------------------------------- | ----------------------------------------------------------- |
| **Diff source**| libgit2: `merge-base(HEAD, origin/main)` … `HEAD` | `gh pr diff <n>`, parse unified                  |
| **Overlay**    | none                                    | description, existing review threads, resolution state      |
| **Sink**       | clipboard (formatted plaintext)         | batched GitHub review via `gh api`                          |
| **Verdicts**   | n/a                                     | approve / request changes / comment-only                    |

Both modes yield the same internal `Diff` representation (`File` → `Hunk` → `Line`). The view never knows which mode produced it.

## Diff acquisition

**Branch mode** — pure libgit2:

```
base = merge-base(HEAD, origin/main)   // uses cached origin/main, no network
head = HEAD
diff = git_diff_tree_to_tree(base, head)
```

This shows only changes the branch introduced — `origin/main` moving forward doesn't pollute the diff. No network fetch. If `origin/main` is stale, the user can fetch separately; we don't auto-fetch.

**PR mode** — `gh` CLI:

```
gh pr view <n> --json headRefName,baseRefName,title,body,headRefOid
gh pr diff <n>            // unified diff text
gh api repos/{o}/{r}/pulls/<n>/comments    // existing line comments
gh api graphql -f query='…'  // resolved-thread state
```

Parse the unified diff into the same `Diff` types. No libgit2 in PR mode.

## Review draft (data model)

A single `Draft` struct holds everything you've authored this session:

```
Draft {
    new_comments: Vec<NewComment>      // (file, line, body)
    replies:      Vec<Reply>           // (thread_id, body)
    resolutions:  Vec<ThreadId>        // threads to mark resolved
    verdict:      Option<Verdict>      // PR mode only
}
```

Branch-mode drafts are just `new_comments` — the other fields stay empty.

The draft is the single mutable thing in the app. Everything else (diff, overlay) is immutable for the session.

## Sinks

### Branch mode — clipboard

Pressing `y` formats the draft as plain text and copies it. Format:

```
src/app.rs:142  (in fn handle_key)
> self.git.diff().unwrap()
COMMENT: unwrap on user input — return Result instead

src/foo.rs:23  (in fn parse_input)
> pub fn parse_input(s: &str) -> Foo {
COMMENT: rename to parse_user_input
```

A few lines of context above each anchor line. Plaintext, not markdown — agents ingest plaintext just as well, and there's nothing to escape.

### PR mode — batched GitHub review

Pressing `Enter` opens a verdict picker. On confirm, one POST to:

```
POST /repos/{o}/{r}/pulls/{n}/reviews
  { event: "APPROVE" | "REQUEST_CHANGES" | "COMMENT",
    body:  "" or top-level review body,
    comments: [ { path, line, body }, … ] }
```

Replies and resolutions follow as separate calls (replies via the comments endpoint, resolutions via GraphQL `resolveReviewThread`).

If any leg of the apply fails, the draft remains intact in the UI so the user can retry.

## PR overlay

Only present in PR mode. Three pieces:

- **Description header** at the top of the scroll, above the first file. Folded by default — `Space` on the header expands.
- **Existing threads** rendered inline, anchored to their line, ordered by author and timestamp. Outdated threads (line moved/gone after a force-push) show with an `[outdated]` marker and don't accept replies.
- **Verdict picker** — modal opened on `Enter`. Three options, default to "comment."

## Comment anchoring

Comments anchor to `(file_path, line)` in the **new** version of the file. Simple, matches GitHub's model, survives re-opens because the diff is recomputed each session.

When a draft's anchor line no longer exists (file edited between sessions), the draft is shown as **stale** in the UI — visible, ignored on apply, dismissible with `d`. The tool doesn't try to reanchor.

For the branch-mode clipboard format, the surrounding code excerpt is generated at copy time from the current diff — so the agent always sees fresh context, never a stale snapshot.

## Folder structure

Vertical slices by domain. UI is read-only over a central state.

```
src/
├── main.rs                # arg parsing, terminal setup, hands off to app
├── app/                   # CENTRAL STATE — single source of truth
│   ├── mod.rs
│   ├── state.rs           # State struct (owns everything below)
│   └── action.rs          # Action enum + dispatch (only place mutations happen)
├── session/               # WHERE diff comes from + WHERE draft goes
│   ├── mod.rs             # Session enum: Branch | Pr
│   ├── branch.rs          # libgit2 + clipboard sink
│   └── pr.rs              # gh CLI + GitHub review sink + overlay loader
├── diff/                  # pure git-diff representation
│   ├── mod.rs
│   ├── types.rs           # File, Hunk, Line
│   ├── compute.rs         # libgit2 ref-pair → Diff (branch mode)
│   └── parse.rs           # unified-diff text → Diff (PR mode)
├── review/                # local draft state
│   ├── mod.rs
│   ├── types.rs           # Draft, NewComment, Reply, Verdict, ThreadId
│   └── format.rs          # clipboard formatter
└── ui/                    # VISUALIZATION LAYER (read-only)
    ├── mod.rs
    ├── render.rs          # the stacked diff render
    ├── syntax.rs          # syntect wrapper + per-file cache
    ├── scroll.rs          # viewport math, navigation
    ├── fold.rs            # collapsed-files set + toggle logic
    ├── status.rs          # bottom status line
    ├── help.rs            # `?` overlay
    ├── theme.rs           # named styles (no magic colors elsewhere)
    └── input.rs           # comment authoring modal
```

## Domain boundaries

```
       ui ──────────────► reads &State, never mutates
        │
        ▼
       app  ◄──► session  (loads Diff & Overlay, delivers Draft)
        │
        ├── owns ──► review (Draft)
        └── holds ──► diff::types (Diff is computed once per session)

   session ──uses──► diff::compute  (branch)
   session ──uses──► diff::parse    (PR)
   session ──uses──► review::format (branch sink: clipboard)
```

Hard rules:

- **`ui/` is read-only.** Receives `&State`, returns events. No git, no `gh`, no clipboard, no network.
- **`app/` is the only mutator.** Every change is an `Action` dispatched through one function. Read anywhere, mutate in one place.
- **`session/` is the I/O boundary.** Only place that touches git, `gh`, network, or clipboard.
- **`diff/` and `review/` are pure.** No I/O, no ratatui, no `gh`. Self-contained, unit-testable in isolation.
- **No `if pr_mode` branches.** Mode-specific behavior lives in `session/branch.rs` or `session/pr.rs`. Everywhere else, the code asks "is there an overlay?" — not "what mode are we in?"

## Keymap

### Movement (vim-canonical)

| Key             | Action                            |
| --------------- | --------------------------------- |
| `j` / `k`       | Line down / up                    |
| `Ctrl-d` / `Ctrl-u` | Half page                     |
| `Ctrl-f` / `Ctrl-b` | Full page                     |
| `g g` / `G`     | Top / bottom                      |
| `] c` / `[ c`   | Next / previous change hunk       |
| `] f` / `[ f`   | Next / previous file header       |
| `] r` / `[ r`   | Next / previous review thread     |

### Folding

| Key      | Action                                                  |
| -------- | ------------------------------------------------------- |
| `Space`  | Toggle file under cursor                                |
| `z`      | Toggle all (anything open → close all, else open all)   |

### Review actions

| Key       | Action                                              |
| --------- | --------------------------------------------------- |
| `c`       | New comment on line under cursor                    |
| `r`       | Reply to thread under cursor                        |
| `R`       | Toggle "resolved" on thread                         |
| `e`       | Edit draft under cursor                             |
| `d`       | Delete draft under cursor                           |
| `o`       | Open file in `$EDITOR` at the relevant line         |
| `y`       | Yank review to clipboard (branch mode)              |
| `Enter`   | Apply review (PR mode) — opens verdict picker       |

In the verdict picker: `a` approve, `x` request changes, `Enter` comment-only, `Esc` cancel.

### Modal / global

| Key      | Action                                  |
| -------- | --------------------------------------- |
| `?`      | Help                                    |
| `Esc`    | Close modal / cancel input              |
| `q`      | Quit (warns if drafts unsaved)          |
| `Ctrl-c` | Force quit                              |

### Reserved (deferred — don't bind)

`/` search · `n` / `N` next / prev match · `:` command mode

## Open file in editor — details

`o` opens `$EDITOR` (fallback `hx`) at a line.

| Cursor on             | Opens at                                        |
| --------------------- | ----------------------------------------------- |
| File header           | Line 1                                          |
| Context line or `+`   | The new line number                             |
| `-` line              | Next surviving line in the new version          |
| Existing thread       | Thread anchor line                              |
| Draft                 | Draft anchor line                               |

Editor command construction: `$EDITOR <path>:<line>` with a small lookup table for editors that don't accept that form (`vim +line file`, `code -g file:line`, etc.).

In PR mode, the file on disk may not match the PR's head ref if the user is on a different branch. The status line shows a warning when `HEAD != session.head_ref`. We don't auto-checkout.

TUI suspend/resume follows the standard ratatui pattern: leave alternate screen, spawn editor synchronously, wait, re-enter alternate screen, redraw.

## Status line

Always visible, bottom of screen.

**Branch mode:**
```
feat/foo → origin/main (merge-base abc1234) · 8 files · 3 drafts ·  142/890
```

**PR mode:**
```
PR #142 · feat/foo → main · 8 files · 3 drafts · resolved 1 ·  142/890
```

If `HEAD != session.head_ref` in PR mode, prepend `⚠ branch:HEAD ` to flag the mismatch.

## Theming

Single dark theme to start. `ui/theme.rs` defines named styles — `DiffAdd`, `DiffRemove`, `HunkHeader`, `FileHeader`, `CommentAuthor`, `DraftMarker`, `Outdated`, `Resolved`, etc. The render layer references names; nothing else hardcodes colors. Configurable themes are deferred until there's a second theme.

## Out of scope (deferred)

These are real features that aren't in v1. Each has a clean place to slot in later — none require structural changes.

- **Search** (`/`, `n`, `N`) — useful but not load-bearing for the loop.
- **Intra-line word diff** — highlight just the changed words within a `-` / `+` pair. Requires a separate diff algorithm; add when missed.
- **Whitespace toggle** — only matters on noisy diffs.
- **Hunk-level folding** — file + thread fold is enough.
- **Line wrapping** — truncate long lines for v1, add wrap if needed.
- **Multiple themes / config file** — add when there's a second theme.
- **Auto-fetch** — branch mode never fetches; PR mode fetches via `gh` only what's needed. No background refresh.
- **Persistent drafts across sessions** — drafts live in memory only. Re-opening on the same branch starts fresh.
- **Comment reanchoring across edits** — stale drafts get marked, not migrated.

## Open questions

None right now. Spec is locked at this commit.
