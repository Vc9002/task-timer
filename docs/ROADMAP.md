# TaskTimer Roadmap

## Current status — 2026-09-16

The v0.3.3 productivity workflow and the next academic-planning pass are on
`main`. The current commit is `b10bdad` (`feat: add academic planning
workflow`). The browser shell has been smoke-tested; browser preview cannot
exercise Tauri commands or the menu-bar process.

Automated evidence currently passes:

- `npm run check`
- `npm run build`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --lib` — 61 passed

The previous baseline was `770ccfa`. The current committed baseline is
`b10bdad`.

## Current planning pass — Study Blocks, Risk, Milestones, Templates

### Implemented on `main`

- Study Blocks are separate planning allocations with date, optional start
  time, duration, edit/delete, and task-level coverage calculations.
- Week and Calendar expose Study Blocks separately from Planned Tasks and Due
  Tasks.
- Incomplete tasks expose derived scheduled minutes before due, planning gap,
  and schedule coverage percentage. This is deterministic schedule coverage,
  not a probability.
- Task planning includes milestone create/edit/complete/reorder/delete.
- Task Templates can be created in Settings and instantiated from Quick Add.

### Known limitations

- Study Blocks can currently overlap. Saving an overlap is allowed, but the
  UI should warn about the conflict before this pass is considered fully
  polished.
- A block's `completed` state means the planned block was acknowledged, not
  that the task or an associated timer session was completed. UI copy should
  use “Mark block done” to keep those states distinct.
- Template milestone presets are supported by the backend instantiation path,
  but there is not yet a dedicated editor for managing them in Settings.

### New migrations

- `0009_study_blocks`
- `0010_task_milestones`
- `0011_task_templates`

### Native validation still open

The implementation and automated checks are complete. Before calling this
pass operationally released:

1. Create multiple blocks for one task and verify they appear in Week and
   Calendar without changing `scheduled_date`.
2. Verify planning-gap math against a task with tracked time and blocks both
   before and after its due date.
3. Create, edit, complete, reorder, and delete milestones.
4. Create a template, instantiate it from Quick Add, and verify its defaults.
5. Run the automated checks, then perform the native macOS check on an
   existing database.

The native Tauri packaging/link step has not produced a new verified bundle in
this pass. CI is intentionally non-blocking for the next development steps;
local checks and native runtime validation remain the meaningful gates.

## Open roadmap — prioritized

### Pass B — Time intelligence and factual review

1. Estimate-vs-actual backend fields and reporting.
2. Estimate accuracy grouped by class and task type, using medians and sample
   thresholds.
3. User-approved estimate suggestions.
4. Planned Study Block minutes versus aggregate actual task/day time.
5. Weekly Review with tracked time, completions, overdue work, estimate misses,
   class totals, session statistics, and schedule coverage.
6. Deadline-risk and workload summaries using remaining work, blocks, capacity,
   and due dates.

Stop after this pass and validate the statistics against real coursework.

### Pass C — Deterministic assisted planning

1. Plan My Day with preview, adjust, apply, and cancel actions.
2. Plan My Week with deadline and capacity constraints.
3. Propose Study Blocks for large assignments without creating fake subtasks.
4. Preserve manual blocks and never schedule work after its deadline.

### Pass D — Integration hardening

1. Todoist outbox status, retry, and last-error UI.
2. Explicit ownership enforcement for every synchronized field.
3. Canvas read-only import with stable course and assignment links.

### Pass E — Semester-scale workflows

1. Semester dashboard.
2. Exam Mode.
3. Deterministic natural-language capture.
4. Search improvements only if indexed search becomes a measured bottleneck.
5. Optional AI proposals only after deterministic planning is reliable.

Do not start Canvas, AI, cloud sync, or additional productivity primitives
before Pass B is validated.

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
