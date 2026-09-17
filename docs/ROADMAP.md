# TaskTimer Roadmap

## Current status — 2026-09-16

The v0.3.3 productivity workflow is implemented in the current working tree,
but it is not yet a release: the changes are uncommitted and native macOS
real-use validation is still required. The browser shell has been smoke-tested;
browser preview cannot exercise Tauri commands or the menu-bar process.

Automated evidence currently passes:

- `npm run check`
- `npm run build`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --lib` — 58 passed

The stable baseline before this working-tree pass is commit `bb14332`.

## v0.3.3 — Productivity workflow

### Implemented

- Cmd/Ctrl+K command palette for views, classes, active tasks, Quick Add, and
  timer start/switch.
- Class-level timers alongside individual task timers; class sessions remain
  separate from coursework while contributing to class analytics.
- Inbox for active, incomplete, unscheduled work.
- Week parent breadcrumbs and Calendar date rescheduling.
- Recurring-template editing with safe regeneration of future occurrences.
- Local task types and normalized tags (`assignment`, `reading`,
  `problem_set`, `exam`, `project`, `other`). Todoist-owned metadata remains
  read-only.
- Per-task time-budget metadata and display. Budgets are targets, not hard
  timer cutoffs or analytics yet.
- Local-task duplication and completion undo. There is no general deletion
  undo stack.
- Focus Mode, persisted locally in the desktop UI.
- Menu-bar elapsed label refreshed once per minute while a timer is active.
- Migration `0007_productivity_workflow` for the new task metadata.

### Release gate

Before calling v0.3.3 released:

1. Run the native Tauri app on macOS with an existing database.
2. Create and edit a local task with type, tags, estimate, and budget.
3. Duplicate it, complete it, and use completion undo.
4. Move a task from Calendar and verify the persisted scheduled date.
5. Toggle Focus Mode and verify restart persistence.
6. Confirm the menu-bar label updates after a minute and remains correct
   across pause/resume and finish.
7. Run the same automated checks in CI, then commit the reviewed working tree.

The browser shell check is useful for layout/navigation only; it does not
close this native release gate.

## v0.4 — Integrations

### Already implemented

- Todoist completion write-back through a durable, retry-safe local outbox.
- Explicit local-vs-Todoist ownership for completion and scheduling behavior.

### Remaining

- Show outbox retry state and last error in the UI.
- Document and enforce ownership for every synchronized field.
- Canvas read-only import with stable course and assignment links.

Canvas may own assignment name, course, and due date. TaskTimer must retain
study scheduling, estimates, subtasks, and time history. Canvas must never
overwrite those local planning fields.

## v0.5 — Time intelligence

After real-use validation, add deterministic reporting for estimate accuracy,
actual-vs-estimated time, session length, workload accuracy, deadline risk,
and weekly review. Use the existing session ledger; do not add AI to the core
workflow.

## v0.6 — Assisted planning

Only after v0.5 evidence is useful:

- Plan My Day / Plan My Week
- workload balancing and task splitting
- optional duration predictions
- optional AI suggestions that always require user approval

Automatic planning must not silently create or reschedule work.

## Explicitly out of scope

Canvas write access, full two-way Todoist field sync, cloud sync, mobile,
Pomodoro/gamification, giant analytics libraries, local LLMs, and AI as a
required core dependency.

## Operating rules

- Treat roadmap status as an evidence claim: source, tests, and runtime checks
  outrank stale prose.
- Preserve the separation between `due_at` and `scheduled_date`.
- Keep external-provider fields fail-closed and local-owned planning fields
  protected from overwrite.
- After each major pass, use the app for several days before adding schema or
  speculative automation.

## macOS hardening backlog

- Validate sleep/wake and screen-lock timer recovery in the native app.
- Keep idle CPU near zero; tray elapsed updates should remain minute-based.
- Continue monitoring WAL growth and use `PRAGMA optimize` periodically.
- Keep Analytics and Settings routes code-split as the app grows.
