# How TimeCop Calculates Diffs

TimeCop aims to show the same diffs as GitHub PR reviews. This document explains how.

## Base Branch Detection

TimeCop prefers remote over local to match GitHub:

```
Priority order:
1. origin/main
2. origin/master
3. main (local)
4. master (local)
```

This ensures diffs stay accurate even when your local main is behind.

## Merge Base

All diffs are calculated relative to the **merge-base** — the common ancestor where your branch diverged from main.

```
         main
           │
     A─────B─────C─────D      ← origin/main
           │
           └──E──F──G──H      ← your branch (HEAD)
              │
              └─ merge-base (commit B)
```

**Diff "all"** = B → H (everything since you branched)

## First-Parent Traversal

When main is merged INTO your branch, GitHub ignores those merge commits. TimeCop does too.

### Without first-parent (wrong):
```
     main
       │
 A─────B─────C─────D
       │           │
       └──E──F─────M──G──H    ← M is merge of D into branch
                   │
                   includes C, D in count (wrong!)
```

### With first-parent (correct):
```
     main
       │
 A─────B─────C─────D
       │           │
       └──E──F─────M──G──H
          │        │
          only counts E, F, M, G, H (correct!)
```

TimeCop uses `simplify_first_parent()` to walk only the main line of your branch.

## Timeline Positions

```
Timeline (left to right):
T─I─M─E─C─O─P─○─○─○─○─[all]─[all+]─[wip]
              │ │ │ │   │      │      │
              │ │ │ │   │      │      └─ uncommitted changes (HEAD → workdir)
              │ │ │ │   │      └──────── all + suggested files (co-change analysis)
              │ │ │ │   └──────────────── all commits (merge-base → HEAD)
              │ │ │ └────────────────── -1 (HEAD~1 → HEAD)
              │ │ └──────────────────── -2 (HEAD~2 → HEAD~1)
              │ └────────────────────── -3 (HEAD~3 → HEAD~2)
              └──────────────────────── -4 (HEAD~4 → HEAD~3)
```

### Position: `wip`
```
Uncommitted changes only

     HEAD ─────────── Working Directory
       │                    │
       └── diff shows ──────┘
```

### Position: `all`
```
All committed changes since branching

  merge-base ─────────── HEAD
       │                   │
       └── diff shows ─────┘
```

### Position: `all+`
```
All committed changes + suggested related files

Same diff as `all`, but the file list also includes
files that frequently change together with your modified
files (based on co-change analysis of recent history).

Suggested files are marked with ◇ in the file list.
```

### Position: `-1`
```
Most recent commit only

    HEAD~1 ─────────── HEAD
       │                 │
       └── diff shows ───┘
```

### Position: `-N`
```
Single commit at offset N

   HEAD~N ─────────── HEAD~(N-1)
       │                   │
       └── diff shows ─────┘
```

## Comparison with GitHub

| Aspect | GitHub PR | TimeCop |
|--------|-----------|---------|
| Base branch | Remote (always) | Remote preferred |
| Merge-base | Computed | Computed |
| First-parent | Yes | Yes |
| Merge commits from main | Hidden | Hidden |
| Uncommitted changes | N/A | Separate "wip" view |

## Commit Offset Calculation

When navigating to `-N`, TimeCop walks first-parent to find the commit:

```rust
// Simplified logic
revwalk.simplify_first_parent();
revwalk.push(HEAD);

for (i, commit) in revwalk.enumerate() {
    if i == N {
        return commit;
    }
}
```

This ensures `-1` is always your most recent commit, not a merge from main.

## Edge Cases

### Stale origin/main
If you haven't fetched recently, diffs may differ from GitHub:
```bash
git fetch origin main
```

### No commits yet
If branch has no commits since branching:
```
T─I─M─E─C─O─P─[all]─[wip]
              (no dots)
```

### Rebased branch
After rebase, merge-base is recalculated automatically. Diffs will be correct.

### Squashed commits
Each squash appears as one commit in the timeline. First-parent handles this correctly.
