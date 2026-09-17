# TaskTimer Roadmap

## Current status — 2026-09-16

The v0.3.3 productivity workflow and the academic planning/intelligence pass
are implemented in the current release batch. The browser shell has been
smoke-tested; browser preview cannot exercise Tauri commands or the menu-bar
process.

Automated evidence currently passes:

- `npm run check`
- `npm run build`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --lib` — 61 passed

The previous baseline was `770ccfa`; the current release batch adds the
planning, intelligence, deterministic planner, semester, and exam slices.

## Implemented release batch

### Planning primitives

- Study Blocks are separate planning allocations with date, optional start
  time, duration, edit/delete, and task-level coverage calculations.
- Week and Calendar expose Study Blocks separately from Planned Tasks and Due
  Tasks.
- Incomplete tasks expose derived scheduled minutes before due, planning gap,
  and schedule coverage percentage. This is deterministic schedule coverage,
  not a probability.
- Task planning includes milestone create/edit/complete/reorder/delete.
- Task Templates can be created in Settings and instantiated from Quick Add.

### Planning polish

- Study Block overlap warnings are non-blocking; overlapping plans can still
  be saved intentionally.
- The UI calls the planning state “Mark block done” so it is not confused with
  task completion or finished timer sessions.
- Template milestone presets have a dedicated Settings editor and instantiate
  into dated task milestones when a due date is supplied.
- The default Start / Switch shortcut is now Cmd/Ctrl+Shift+Y; existing users
  with the shipped Cmd/Ctrl+Shift+T default are migrated, while custom choices
  are preserved.

### New migrations

- `0009_study_blocks`
- `0010_task_milestones`
- `0011_task_templates`
- `0012_exams`

### Pass B — time intelligence

- Estimate-vs-actual reporting with median error and sample counts.
- Accuracy breakdowns by class and task type.
- User-approved estimate suggestions with a three-sample minimum.
- Aggregate Study Block planned minutes versus tracked task time.
- Weekly Review with tracked time, completions, overdue work, estimate misses,
  session statistics, schedule coverage, and daily capacity/deadline load.

### Pass C — deterministic assisted planning

- Plan My Day and Plan My Week proposal commands.
- Preview, adjust, cancel, and explicit Apply flow.
- Capacity-aware Study Block splitting with a 25-minute minimum and 90-minute
  chunk preference.
- Existing blocks are preserved; overdue tasks are surfaced instead of being
  silently scheduled after their deadline.

### Pass D — integration hardening

- Todoist outbox status is visible as Synced, Queued, Retrying, or Failed.
- Failed completion delivery exposes a Retry action and last error.
- Existing local-first and field-ownership behavior remains unchanged.

### Pass E — semester-scale workflows

- Semester dashboard with tracked time, completed/open tasks, and due-soon
  workload by class.
- Exam Mode creates an ordinary exam-prep task, so timer history and Study
  Blocks remain unified.
- Deterministic shorthand capture in Quick Add for class, estimate, due day,
  and today/tomorrow scheduling.

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
this pass. CI is intentionally non-blocking; local checks and native runtime
validation remain the meaningful gates.

## Open roadmap — prioritized

### Remaining roadmap

1. Native macOS real-use validation of the new planning, analytics, planner,
   and exam flows; the new bundle must still be built and exercised against an
   existing database.
2. Canvas read-only integration. This is blocked until the Canvas base URL,
   authentication method, and ownership mapping are explicitly configured; no
   credentials or institution endpoint are hardcoded.
3. Optional AI proposals. This is blocked until an AI provider/model and
   credential boundary are explicitly chosen; deterministic planning remains
   fully usable without it.
4. Search upgrades only if measured scale makes indexed search necessary.

Do not add another integration or dashboard until the native validation and
real-coursework review have been completed.

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
