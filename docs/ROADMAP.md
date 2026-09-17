# TaskTimer Roadmap

## Current status — 2026-09-17

v0.3.3 productivity + academic planning/intelligence pass, including the Pomodoro feature, is implemented and native-validated. App version is tagged 0.3.3 across `package.json`, `Cargo.toml`, and `tauri.conf.json` (previously stuck at `0.1.0`).

Native validation (2026-09-17): release binary built via `cargo build --release`, launched against the real production database (backed up first to `~/task-timer-db-backups/`), ran stable with zero stderr/stdout errors, then shut down cleanly. Database integrity check passed before and after. Interactive walkthrough of the checklist (blocks, milestones, templates, exam, Pomodoro cycle) confirmed good by the user.

Automated evidence currently passing:

- `npm run check` — 202 files, 0 errors, 0 warnings
- `npm run build`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo test --lib` — 77 passed

Code review (2026-09-16): no outstanding issues. SQL is parameterized throughout, error handling is consistent (`Result<_, String>` at the command boundary), no debug leftovers, no TODOs/FIXMEs. Full findings below under "Code review notes."

Previous baseline: `770ccfa`. Current release batch adds planning, intelligence, deterministic planner, semester, and exam slices on top of that baseline.

## Implemented release batch

### Planning primitives

- Study Blocks: separate planning allocations from a task's own `scheduled_date` (date, optional start time, duration, edit/delete, task-level coverage calculations).
- Week Calendar exposes Study Blocks separately from Planned Tasks and Due Tasks.
- Incomplete tasks expose derived scheduled minutes, due date, planning gap, and schedule coverage percentage. This is deterministic schedule coverage, not a probability estimate.
- Task planning includes milestone create/edit/complete/reorder/delete.
- Task Templates are created in Settings and instantiated from Quick Add.

### Planning polish

- Study Block overlap warnings are non-blocking; overlapping plans can still be saved intentionally.
- UI calls the planning-state action "Mark block done" so it isn't confused with task completion or finished timer sessions.
- Template milestone presets have a dedicated Settings editor and instantiate into dated task milestones when a due date is supplied.
- Default Start/Switch shortcut is now Cmd/Ctrl+Shift+Y; existing users previously shipped with Cmd/Ctrl+Shift+T are migrated automatically, while custom choices are preserved.

### New migrations

- `0009_study_blocks`
- `0010_task_milestones`
- `0011_task_templates`
- `0012_exams`

### Estimate-vs-actual (Pass C)

- User-approved suggestions require a three-sample minimum before surfacing.
- Aggregates Study Block planned minutes versus tracked task time.
- Weekly Review reports tracked time against capacity and deadlines.
- Capacity-aware scheduling in 25-minute and 90-minute increments; existing blocks are preserved, and overdue tasks are surfaced instead of silently rescheduled past their deadline.

### Pass D — integration hardening

- Todoist outbox status is visible in the UI: Synced, Queued, Retrying, Failed.
- Failed completion delivery exposes a Retry action and the last error.
- Existing local-first field-ownership behavior is unchanged.

### Pass E — semester-scale workflows

- Semester dashboard: tracked time, completed/open tasks, due-soon workload by class.
- Exam Mode creates an ordinary exam-prep task, so timer history and Study Blocks remain unified with the rest of the app.
- Deterministic shorthand capture in Quick Add for class, estimate, due day, and today/tomorrow scheduling.

### Pomodoro (2026-09-16)

- Standalone Pomodoro clock, available from the sidebar for whatever task you're on — not a new time-tracking concept, doesn't touch `time_sessions`.
- Classic work/break cycle with user-configurable lengths (default 25/5 minutes), edited from Settings.
- When a task timer is actively running, the Pomodoro clock auto-pauses it going into a break and auto-resumes it coming out, only if it paused it in the first place; manual pauses are left alone.
- Local notification on each phase change (work done / break's over) via the existing notification plugin; silently no-ops if notification permission isn't granted.
- Settings persisted in `app_settings` under `pomodoro_v01` — no new migration needed.
- Backend: `src-tauri/src/commands/pomodoro.rs` (3 unit tests: defaults, round-trip, range validation). Frontend: `pomodoroStore` (`src/lib/stores/pomodoro.svelte.ts`), sidebar widget in `+layout.svelte`, `PomodoroSettings.svelte`.

## Code review notes (2026-09-16)

Full manual review of the Rust backend (~7,200 lines) and Svelte frontend (~2,900 lines), since the working tree had no diff to review incrementally.

- No `unwrap`/`expect`/`panic!` outside tests, except two justified fail-fast calls at app startup in `lib.rs` (resolving the app data dir and opening the database) — correct behavior for boot-time failures.
- No TODO/FIXME/HACK markers anywhere in the codebase.
- No stray `console.log`/`dbg!`; all `eprintln!` calls are legitimate error logging in background paths (tray refresh, reminders, outbox recovery, Todoist sync, today digest).
- SQL uses parameterized queries throughout; spot-checked `exams.rs` and `todoist/sync.rs`.
- Frontend routes for semester, analytics, and exams are fully wired to backend commands — not stubs, despite being visually compact files.
- `docs/ROADMAP.md` had drifted out of sync with the migration list and contained garbled prose from an earlier lossy edit; both fixed in this update.

No functional bugs found. Nothing here blocks native validation.

## Native validation — complete (2026-09-17)

All items below were verified against the real production database via the native release binary:

1. Multiple blocks per task appear in the Week Calendar without changing `scheduled_date`. ✓
2. Planning-gap math checked for a task with tracked time and blocks before/after its due date. ✓
3. Milestone create/edit/complete/reorder/delete. ✓
4. Template creation, instantiation from Quick Add, and defaults. ✓
5. Exam creation, its study task in the timer/planning flow, tracked-minute rollup. ✓
6. Pomodoro cycle with a task timer running: auto-pause into break, auto-resume into next work phase, phase-change notifications delivered. ✓
7. Automated checks plus native macOS run against the existing (non-empty) database. ✓

This pass is operationally released as v0.3.3.

## Feature backlog — complete (2026-09-17)

All nine backlog items from the 2026-09-16 review are implemented, tested, and committed:

1. **Idle detection.** Polls macOS `HIDIdleTime` every 30s, auto-pauses the active timer past a configurable threshold (default 5 min, 0 disables it).
2. **Pomodoro long break.** Every 4th work cycle takes a configurable long break instead of the short one.
3. **Tray countdown for Pomodoro.** Frontend clock pushes phase/remaining time to the tray menu every 15s and on phase change.
4. **Task notes.** Free-text `notes` column (migration 0013), editable from the task edit form.
5. **.ics calendar export.** Open task due dates and unfinished Study Blocks as RFC 5545 VEVENTs, alongside CSV/JSON export.
6. **In-app backup/restore.** Restores classes/tasks/time_sessions/todoist_projects from a JSON backup, pruning any planning data left pointing at an id the backup doesn't have. Two-click confirm since it's destructive.
7. **Weekly review nudge.** Configurable day/hour local notification pointing at the existing Analytics weekly review, tracked by ISO week so it fires at most once.
8. **Task dependencies ("blocked by").** New `task_dependencies` table (migration 0014) with a recursive-CTE cycle check; a `blocked_by_open_count` field rides along on every task query via the shared `TASK_SELECT` projection.
9. **Tags filter.** Tags already existed as a class-independent field on tasks; added a client-side filter to Inbox rather than rebuilding what was already there.

All verified via `cargo test --lib` (77 passing), `cargo clippy -D warnings`, `cargo fmt --check`, and `npm run check` (202 files, 0 errors) after each addition. Native macOS walkthrough of these specific features has not yet happened — see Next steps.

## Product direction

Local-first academic planning and time intelligence for macOS, with optional Todoist interoperability. TaskTimer stays focused on four things: capture work, plan work, track actual time, and improve future planning from your own history. Canvas and AI are non-goals — see "Explicitly out of scope."

## Next steps (in order)

1. **Native macOS validation of the 2026-09-17 feature batch.** The items above are automated-checks-clean but haven't been walked through in the real `.app` the way the 2026-09-16 planning/exam/Pomodoro pass was. Particularly: idle auto-pause (needs real inactivity, can't be simulated), tray countdown rendering, and a real backup/restore round-trip against a copy of the production database.
2. **Performance hardening.** Activity Monitor audit of the running app, reduce background wakeups, inspect SQLite query plans, eliminate N+1 queries.
   - Background wakeup audit (2026-09-17): idle-detection poll is 30s, weekly-review poll is 15min — both sparse and reasonable. The two frontend 1s `setInterval` ticks (task timer, Pomodoro) only run while a session is actively counting, and are torn down otherwise — no idle wakeup cost.
   - N+1 fix (2026-09-17): the Today view and Week view each issued 2 SQL round-trips per task (a per-task `course_code` lookup and a per-task overdue date-compare query). Both now prefetch a class-code map once and batch the overdue computation into a single query per view via a `VALUES`-table join, preserving exact SQLite date-comparison semantics (date-only strings are floating local dates; timestamps go through `'localtime'`). Verified with `cargo test --lib` (77 passing), `cargo clippy -D warnings`, `cargo fmt --check`, `npm run check`.
   - Remaining: `week.rs`'s per-ancestor parent-path walk (one query per level climbing `parent_task_id`) is still per-row but bounded by hierarchy depth, which is typically shallow — left as-is pending evidence it matters. SQLite query-plan (`EXPLAIN QUERY PLAN`) inspection of the larger aggregate queries (analytics, semester dashboard) not yet done.
3. **Planning quality.** Improve Plan My Day/Week heuristics, dependency-aware scheduling, better handling of partially completed Study Blocks, overdue work, exams, and capacity.
   - Dependency-aware scheduling (2026-09-17): the "blocked by" feature (task_dependencies, shipped in the feature backlog pass) was never wired into the planner — `get_plan_proposal` could propose Study Block time for a task still blocked by an incomplete prerequisite. Fixed: the planner now filters on `blocked_by_open_count == 0`, so blocked tasks are excluded from the proposal entirely until their blockers complete. Refactored the Tauri command into a plain `plan_proposal_for(conn, start_date, days)` so it's directly testable (matching the `today_for`/`week_for` pattern), and added two tests covering the blocked/unblocked transition. `cargo test --lib` (79 passing), `cargo clippy -D warnings`, `cargo fmt --check`, `npm run check` all clean.
   - Partially completed Study Blocks were already handled correctly (`existing_minutes` only sums `completed=0` blocks against a day's capacity) — no change needed there.
   - Remaining: heuristic tuning (currently a fixed 25/90-minute chunking with due-date/priority/unplanned-minutes sort — no evidence yet it's suboptimal), exam-specific scheduling weighting, and capacity edge cases (e.g. a day with an override lower than already-committed block time) not yet reviewed.
4. **Analytics quality.** Estimate calibration, weekly review, semester trends, planned-vs-actual, deadline coverage, class workload.
5. **Operational reliability.** Backup/restore verification, migration testing, export validation, crash recovery, sleep/wake behavior.
6. **UX polish.** Keyboard navigation, clearer empty/error states, consistent task/block/completion language, better dense Mac layouts.
7. **Todoist as the only external integration**, with explicit ownership and retry/status UI (already partially implemented — see Pass D above; continue hardening).
8. **Search upgrades** — only if measured scale makes indexed search necessary. Not needed yet.

## Explicitly out of scope

**Canvas integration (read or write), in any form** — removed as a non-goal. TaskTimer is not becoming a general LMS/integration platform. Full two-way Todoist field sync, cloud sync, mobile, Pomodoro streaks/gamification/scoring, giant analytics libraries, local LLMs, and any AI features (proposals, duration predictions, heuristic generation) — decided against; estimates and planning stay fully manual and locally computed.

## Operating rules

- Treat roadmap status as a claim requiring evidence: source, tests, and runtime checks outrank stale prose.
- Preserve the separation between `due_at` and `scheduled_date`.
- Keep external-provider fields fail-closed and local-owned by default.
- Use the app for real coursework for days before adding schema for speculative automation.

## macOS hardening backlog

- Validate sleep/wake and screen-lock timer recovery in the native app.
- Keep idle CPU near zero; tray elapsed updates should remain minute-based.
- Continue monitoring WAL growth and use `PRAGMA optimize` periodically.
- Keep Analytics and Settings routes code-split as the app grows.
