# TaskTimer Roadmap

## Current status (2026-09-16)

The v0.2 desktop release and v0.3 planning release are implemented on
`main`. Todoist completion write-back is implemented as a local, retryable
outbox. This stabilization pass fixes the remaining CI and write-back
correctness issues before further feature work.

Completed: tray controls, global shortcuts, autostart, notifications,
exports, Today/Week/Calendar planning, capacity and workload summaries,
recurring task generation, and Todoist completion queuing.

Still intentionally deferred: Canvas import, full Todoist field write-back,
advanced analytics, automatic scheduling, and AI. These are separate releases
and must not be mixed into the stabilization work.

## Path

```
v0.2 stable desktop app
        |
v0.3   Week + workload planning
        |
v0.3.1 Real-use validation
        |
v0.3.2 Recurring tasks + Calendar
        |
v0.4   Todoist write-back + Canvas
        |
v0.5   Estimate / productivity intelligence
        |
v0.6   Automatic planning + optional AI
        |
v1.0   Stable personal academic OS
```

Rule: after each major pass, stop adding features and actually use the app
for a few days. Real workflow friction surfaces more than speculative
feature code does.

## v0.3 — Academic Planning (done, see commit 092835f)

Pass 1 scope, in order:

1. Week view (`/week`)
2. Direct vs. aggregate tracked time (`tracked_seconds_direct` /
   `tracked_seconds_total`)
3. Remaining-time calculation (`estimated_minutes - direct_tracked_minutes`,
   clamped at zero)
4. Study capacity (optional, per-weekday + date overrides)
5. Workload forecasting (per-day load %, overload warnings)
6. Next Up (Today page, deterministic ordering, no numeric score)
7. macOS CI job

Explicitly deferred to later passes: recurring tasks, calendar view,
command palette, tags, search, Todoist write-back, Canvas import, AI.

Key decisions:
- `due_at` (when it's due) and `scheduled_date` (when you plan to work on
  it) are always kept separate. Week view places tasks by `scheduled_date`.
- Unscheduled tasks surface in Week view when incomplete, unscheduled, and
  due within a 7-day lookahead past the visible week.
- Parent task remaining-time is direct estimate minus direct tracked time
  only — never compared against total descendant time.
- Next Up ordering: overdue, then scheduled today, then soonest due, then
  higher manual priority, then more remaining work, then task id as a
  stable tie-break. No opaque score.
- Priority is standardized to 1 (Low) / 2 (Normal, default) / 3 (High) /
  4 (Urgent). Todoist priority is mapped through an explicit conversion
  function (`src/lib/priority.ts`), never assumed equal to local values.

## v0.3.1 — Planning validation polish

Use the app for real coursework for a few days before adding more schema.
Verify: remaining-time math, parent/subtask aggregation, overloaded-day
warnings, Next Up ordering, Mac sleep/wake, menu-bar behavior, workload
calculations. Add tests for week/month/year boundaries. Keep macOS CI green.

## v0.3.2 — Recurring tasks + Calendar (implemented; stabilization ongoing)

- Recurring task templates (`recurring_task_templates`, new migration,
  never edit old migrations)
- Generated occurrences as individual rows (never reuse one row), lazily
  generated ~30 days ahead: on startup, when a template is edited, when
  Week page opens
- Editing a template only touches future, unstarted, generated
  occurrences — never rewrites completed history
- Calendar view built on the same scheduling model as Week, CSS grid only
  (no calendar library), always separating planned work from due dates

## v0.4 — Integrations (completion write-back implemented; Canvas deferred)

- Todoist completion write-back via a persistent, retry-safe outbox.
  Not every field two-way immediately.
- Canvas read-only import: Canvas owns assignment name/course/due date;
  TaskTimer owns study scheduling, estimates, subtasks, timing. Canvas
  never overwrites local planning metadata.

## v0.5 — Time intelligence

Estimate-vs-actual analytics, task types, median duration by task
type/class, weekly reports, session-length stats, workload accuracy,
suggested estimates. Statistical and deterministic — no AI required.

## v0.6 — Automatic planning + optional AI

Predicted durations, "Plan My Week," automatic workload balancing,
splitting large assignments across days. Only after that: optional AI
explanations / task-breakdown suggestions, always requiring user approval
before creating anything. AI is never required for core app function.

## Deferred features (post-Week-view backlog)

Scoped for Passes 2–6 once the planning core is validated:

- **Pass 2**: recurring tasks, calendar, task types, tags, search
  (indexed SQLite, no heavy search library), command palette (`⌘K`), inbox
- **Pass 3**: study blocks, milestones, templates, per-task time budgets,
  focus mode, menu-bar mini dashboard
- **Pass 4**: advanced analytics — estimate accuracy, deadline-risk,
  context switching, weekly review
- **Pass 5**: Todoist write-back, Canvas read-only import
- **Pass 6**: Plan My Day / Plan My Week, automatic duration predictions,
  optional AI

Highest-value additions beyond the core plan, in no particular order:
command palette, inbox, study blocks, time budgets, milestones, search,
templates, deadline risk.

## Explicitly out of scope until stated otherwise

Canvas write access, AI required for core function, Todoist full two-way
sync on first pass, cloud sync, mobile, Pomodoro/gamification, giant
analytics chart libraries, drag-and-drop calendar, local LLM.

## macOS-specific priorities

- macOS CI job (done)
- Lazy-load Analytics/Settings routes
- Bulk SQL over per-row DB calls; indexes on `tasks(status)`,
  `tasks(due_at)`, `tasks(scheduled_date)`, `tasks(class_id, status)`,
  `tasks(parent_task_id)`, `time_sessions(task_id, start_ts)`,
  `time_sessions(start_ts)` (done in 0004 migration)
- Sleep/wake and screen lock/unlock correctness for the persistent timer
- Idle CPU target ~0%; menu bar updates once per minute, not every second
- `PRAGMA optimize` periodically; avoid forced `VACUUM` on every launch;
  monitor WAL growth
