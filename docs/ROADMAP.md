# TaskTimer Roadmap

## Current status — 2026-09-17

v0.3.3 productivity + academic planning/intelligence pass, including the Pomodoro feature, is implemented and native-validated. App version is tagged 0.3.3 across `package.json`, `Cargo.toml`, and `tauri.conf.json` (previously stuck at `0.1.0`).

Native validation (2026-09-17): release binary built via `cargo build --release`, launched against the real production database (backed up first to `~/task-timer-db-backups/`), ran stable with zero stderr/stdout errors, then shut down cleanly. Database integrity check passed before and after. Interactive walkthrough of the checklist (blocks, milestones, templates, exam, Pomodoro cycle) confirmed good by the user.

Automated evidence currently passing:

- `npm run check` — 200 files, 0 errors, 0 warnings
- `npm run build`
- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `cargo test --lib` — 64 passed

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

## Next steps (in order)

Small, bounded — no new subsystems:

1. **Idle detection.** Auto-pause the timer after N minutes of no keyboard/mouse activity, so stepping away doesn't silently track time.
2. **Classic Pomodoro long break.** Every 4th work cycle triggers a longer break (15–20 min) instead of the short one.
3. **Tray countdown for Pomodoro.** Mirror the sidebar widget's remaining time in the menu-bar tray label alongside the existing timer status.
4. **Task notes.** Free-text notes field per task for context beyond the title — one column, one textarea, no new subsystem.

Medium — touches existing flows, still bounded:

5. **.ics calendar export.** Export Study Blocks/due dates to a calendar file, alongside the existing CSV/JSON export.
6. **In-app backup/restore.** `export_data` already exists; add a restore-from-backup command so a bad state is recoverable without manual sqlite file surgery.
7. **Weekly review nudge.** Scheduled local notification pointing at the existing Analytics weekly review, so it doesn't rely on remembering to check.

Larger — needs its own brainstorm/design pass, real schema changes:

8. **Task dependencies** ("blocked by" / "blocks") — useful for multi-step assignments.
9. **Tags/labels independent of class** — cross-class groupings (e.g. "reading," "problem set").

Blocked on a config decision from you, no implementation possible yet:

10. **Canvas read-only integration.** Needs Canvas base URL and authentication method. No credentials or institution endpoint should be hardcoded. Canvas should own assignment name, course, and due date; TaskTimer retains study scheduling, estimates, subtasks, and time history. Canvas must never overwrite local planning fields.
11. **Search upgrades** — only if measured scale makes indexed search necessary. Not needed yet.

## Explicitly out of scope

Canvas write access, full two-way Todoist field sync, cloud sync, mobile, Pomodoro streaks/gamification/scoring, giant analytics libraries, local LLMs, AI proposals/duration predictions (decided against — estimates and planning stay fully manual).

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
