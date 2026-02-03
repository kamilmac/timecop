# TODO

Remaining refactors and improvements.

---

## Widget Refactoring

### Remaining Tasks

- [ ] Create `WidgetState` trait in `src/ui/widgets/mod.rs` (optional - for uniformity)

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

---

## Minor Fixes
