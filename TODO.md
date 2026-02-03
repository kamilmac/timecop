# TODO

Remaining refactors and improvements.

---

## Widget Refactoring

### Remaining Tasks

- [ ] Create `WidgetState` trait in `src/ui/widgets/mod.rs` (optional - for uniformity)

### Extract PrDetailsView from DiffView

Separating concerns makes DiffView more focused and scalable:

- [ ] Create `pr_details/` widget for PR info, reviews, general comments
- [ ] Remove `PreviewContent::PrDetails` variant from DiffView
- [ ] Move `parse_pr_details()` to pr_details widget
- [ ] Update App to render PrDetailsView when PR list is focused
- [ ] DiffView becomes diff-only (FileDiff, FolderDiff, Loading, Empty)

---

## Event-Driven Architecture

### Goal
All state changes flow through events. No polling, no special cases. Timers and watchers spawn events.

### Tasks

- [ ] Add new event variants to `AppEvent` (BranchChanged, PrListLoaded, etc.)
- [ ] Make `AsyncLoader` generic (takes event sender)
- [ ] Add `.git/HEAD` to file watcher for branch detection
- [ ] Extract timer logic from `handle_tick()`
- [ ] Create `App.handle_event()` dispatcher
- [ ] Remove polling from `handle_tick()`

---

## Cleanup

### Remaining Audit

- [ ] Run `cargo +nightly udeps` to find unused dependencies
- [ ] Review each widget for unused methods
- [ ] Check for overly complex abstractions that can be simplified

---

## Diff View Modes

### Split/Single View Toggle

- [ ] Add `DiffViewMode` enum (`Split`, `Single`) to DiffViewState
- [ ] Add "s" key binding to toggle between modes
- [ ] Implement single-pane unified diff rendering
- [ ] Persist user preference (or just session state)

### Auto-Switch Based on Width

- [ ] Detect available width in diff preview area
- [ ] Auto-switch to single view when width < threshold (e.g., 120 chars)
- [ ] Auto-switch to split view when width >= threshold
- [ ] Manual toggle should override auto behavior until resize

### Ensure Consistency Across Modes

**Line comments must work in both modes:**
- [ ] Rendering inline comments in split view (current)
- [ ] Rendering inline comments in single/unified view
- [ ] Adding comments on correct line in both modes
- [ ] Comment positioning: left side (old) vs right side (new)

**Editor integration must work in both modes:**
- [ ] `get_current_line_number()` returns correct line for both modes
- [ ] In split: use right-side (new file) line number
- [ ] In single: map unified diff position to actual file line
- [ ] "o" key opens editor at correct line regardless of mode

**Cursor preservation when switching modes:** (depends on focus behavior)
- [ ] Track logical position (file line number) not display position
- [ ] When toggling mode, stay on same file line
- [ ] Handle edge cases: cursor on deleted line (no right-side equivalent)
- Note: design this together with main window focus behavior

---

## Minor Fixes
