---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
<!-- refreshed: 2026-09-20 -->

# Architecture

**Analysis Date:** 2026-09-20

## System Overview

TaskTimer is a Tauri 2 + Svelte desktop application for task/time tracking with student-focused features (classes, exams, study planning). The system is split into a Frontend (SvelteKit SPA) and Backend (Rust), with bidirectional IPC communication via Tauri's invoke mechanism.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                      Frontend Layer (SvelteKit + Svelte)                 │
│  Routes: Today, Inbox, Week, Calendar, Classes, History, Analytics      │
│  `src/routes/`                                                            │
├─────────────────────────────────────────────────────────────┬────────────┤
│  Components         │ State Stores     │ API Wrappers       │ Utilities  │
│  `lib/components/`  │ `lib/stores/`    │ `lib/api.ts`       │ `lib/`     │
│  (Modal, TaskPicker,│ (timer, pomodoro,│ (invoke helpers)   │ (format,   │
│   TaskTree, etc.)   │  theme, updater) │                    │  priority) │
└─────────────────────────────────────────────────────────────┴────────────┘
         │ Tauri IPC invoke() ↔ generate_handler![] │
┌────────▼──────────────────────────────────────────────────────────────────┐
│                    Backend Layer (Rust + SQLite)                          │
│                      `src-tauri/src/`                                      │
├───────────┬─────────────────────────┬──────────────┬──────────────────────┤
│ Commands  │  Subsystems             │ Timer Core   │ Database             │
│           │                         │              │                      │
│ - tasks   │ - todoist (sync)        │ `timer/`     │ `db/mod.rs`          │
│ - classes │ - tray (menu + globals) │ (session mgmt│ Migrations (14)      │
│ - week    │ - idle (auto-pause)     │ pausable)    │ SQLite WAL mode      │
│ - today   │ - desktop (shortcuts)   │              │ Foreign keys ON      │
│ - planner │ - reminders (notify)    │              │                      │
│ - etc.    │ - weekly_review (jobs)  │              │                      │
│           │ - exam_countdown (job)  │              │                      │
│           │ - export/import         │              │                      │
└───────────┴─────────────────────────┴──────────────┴──────────────────────┘
         │ tauri::State<Db> shared Mutex<Connection> │
         ▼
┌────────────────────────────────────────────────────────────────────────────┐
│  SQLite Database (local file)                                              │
│  `~/.tauri/com.vincentc9002.tasktimer/task-timer.sqlite`                   │
│  Tables: tasks, classes, time_sessions, study_blocks, exam_records, etc.   │
└────────────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| **App Layout** | Main shell, sidebar nav, timer bar, modals | `src/routes/+layout.svelte` |
| **Timer Store** | Active session state, pause/resume logic, recovery UI | `src/lib/stores/timer.svelte.ts` |
| **Pomodoro Store** | Work/break cycles, phase transitions, tray sync | `src/lib/stores/pomodoro.svelte.ts` |
| **Theme Store** | Light/dark mode, color variables | `src/lib/stores/theme.svelte.ts` |
| **Updater Store** | App update checking and installation | `src/lib/stores/updater.svelte.ts` |
| **Task Commands** | Create, update, delete, schedule tasks | `src-tauri/src/commands/tasks.rs` |
| **Analytics Commands** | Daily/weekly/monthly stats, streaks, estimates | `src-tauri/src/commands/analytics.rs` |
| **Timer Core** | Start/pause/finish/recover sessions | `src-tauri/src/timer/mod.rs` |
| **Todoist Sync** | Bi-directional sync with Todoist API | `src-tauri/src/todoist/sync.rs` |
| **Idle Detector** | Background process: auto-pause on inactivity | `src-tauri/src/idle.rs` |
| **Desktop Shortcuts** | Global hotkeys (start, pause, finish, quick-add) | `src-tauri/src/desktop.rs` |
| **Tray Menu** | System tray icon, menu items, quick actions | `src-tauri/src/tray.rs` |
| **Database** | SQLite wrapper, migrations, connection pooling | `src-tauri/src/db/mod.rs` |

## Pattern Overview

**Overall:** Tauri IPC + event-driven React-like (Svelte 5) frontend with reactive stores, backed by Rust command handlers that serialize/deserialize JSON and operate on SQLite.

**Key Characteristics:**

- **Desktop-first SPA:** Static adapter + fallback to index.html; no server-side rendering.
- **Single active timer per session:** Conflict modal when user attempts to switch tasks.
- **Bidirectional sync:** Timer changes from tray/shortcuts emit events that update frontend stores; frontend invokes Tauri commands.
- **Background jobs:** Idle monitor, reminders, and weekly review run in separate threads spawned at app startup.
- **Opt-in integrations:** Todoist sync requires explicit token entry; outbox-based queueing for offline safety.

## Layers

**Presentation Layer:**

- Purpose: Render UI, collect user input, display real-time timer.
- Location: `src/routes/`, `src/lib/components/`, `src/lib/stores/`
- Contains: Svelte components (`.svelte`), TypeScript helpers, CSS.
- Depends on: Tauri invoke API, local stores, browser APIs (localStorage, events).
- Used by: Desktop window.

**API/Command Layer:**

- Purpose: Expose backend functions as Tauri commands; parse JSON requests and responses.
- Location: `src-tauri/src/commands/`
- Contains: One `.rs` file per feature domain (tasks, week, pomodoro, todoist, etc.).
- Depends on: Database layer, subsystems.
- Used by: Frontend via `invoke()`.

**Business Logic Layer:**

- Purpose: Core algorithms (timer math, scheduling, analytics, Todoist reconciliation).
- Location: `src-tauri/src/timer/mod.rs`, `src-tauri/src/todoist/`, `src-tauri/src/commands/*.rs`.
- Contains: Session elapsed-time computation, conflict detection, sync retry logic, estimate generation.
- Depends on: Database.
- Used by: Commands.

**Database Layer:**

- Purpose: SQLite schema management, connection pooling, query execution.
- Location: `src-tauri/src/db/mod.rs`, `src-tauri/src/db/migrations/`.
- Contains: 14 schema migrations, WAL mode setup, foreign key enforcement.
- Depends on: rusqlite, chrono, uuid (for data generation).
- Used by: All commands and subsystems.

**Subsystem Layer:**

- Purpose: Background monitoring and integration.
- Location: `src-tauri/src/idle.rs`, `src-tauri/src/reminders.rs`, `src-tauri/src/tray.rs`, `src-tauri/src/desktop.rs`, `src-tauri/src/todoist/sync.rs`.
- Contains: 30-second idle polling, notification dispatch, global shortcut registration, Todoist sync queue recovery.
- Depends on: Database, Tauri plugins (notification, global-shortcut, autostart).
- Used by: Setup in `lib.rs` at app startup.

## Data Flow

### Primary Request Path (User Action → Timer Change)

1. **Frontend:** User clicks "Start task" button → `TaskPicker` component opens (`src/lib/components/TaskPicker.svelte`).
2. **Frontend:** User selects task → calls `timerStore.start(taskId)` (`src/lib/stores/timer.svelte.ts:55`).
3. **Frontend:** `start()` invokes Tauri command `start_timer` with taskId.
4. **IPC:** Tauri deserializes JSON to `commands::tasks::startTimer()` function (`src-tauri/src/commands/tasks.rs`).
5. **Backend:** Command acquires database lock and queries for active session.
6. **Backend:** If conflict (another task running), returns `TimerError::ActiveSessionConflict` with current task info (`src-tauri/src/timer/mod.rs:32`).
7. **Frontend:** Store catches error, renders conflict modal with "Switch" and "Keep Current" buttons.
8. **Backend:** If no conflict, inserts new `time_sessions` row (start_ts, task_id, no end_ts yet).
9. **Backend:** Returns `ActiveSessionInfo` (session, task_title, class_code, elapsed_seconds).
10. **Frontend:** Store updates `active`, starts 1-second tick interval to show elapsed time.
11. **Frontend:** Renders timer bar at bottom with pause/finish buttons.

### Pause/Resume Flow

1. **Frontend or Tray:** User presses Ctrl+Shift+Y (pause) or clicks pause button.
2. **Backend:** `pause_timer` command sets `pause_started_ts = now()`.
3. **Frontend:** Store receives updated session, displays "(paused)" label, stops tick interval.
4. **Frontend/Backend:** Elapsed time is recomputed from stored timestamps on each refresh, never an in-memory counter.

### Analytics Aggregation

1. **Frontend:** User navigates to `/analytics`.
2. **Frontend:** Calls `invoke("get_analytics_week")`.
3. **Backend:** `commands::analytics::get_analytics_week()` queries:
   - All tasks completed/in-progress for the week.
   - For each task, SUM of `final_duration_seconds` from related `time_sessions` rows (including descendants via recursive CTE).
   - Returns structured data: tasks, total minutes, breakdowns by priority/class.
4. **Frontend:** Renders charts and stats using returned data.

### Todoist Sync (Opt-In)

1. **Frontend:** User enters Todoist API token in Settings → `invoke("set_todoist_token", { token })`.
2. **Backend:** Stores token in OS keychain (via `keyring` crate), stores marker in database.
3. **Frontend:** User clicks "Sync now" → `invoke("sync_todoist_now")`.
4. **Backend:** `todoist::sync::sync_all()` fetches projects/tasks from Todoist API.
5. **Backend:** Reconciles with local tasks, creates/updates/deletes local rows, populates outbox for failed writes.
6. **Backend:** Returns sync status (projects mapped, tasks merged, conflicts queued).
7. **Frontend:** Displays outbox items in a tab; user can view and retry failures.

### Idle Detection (Background)

1. **App startup:** `idle::setup()` spawns thread that polls every 30 seconds (`src-tauri/src/idle.rs:6`).
2. **Polling loop:** Checks if active session exists and is not paused.
3. **Polling loop:** Runs `ioreg -c IOHIDSystem -d 4` to read `HIDIdleTime` (nanoseconds since last input).
4. **Threshold check:** If idle time > threshold (e.g., 5 minutes), calls `core_pause_timer()`.
5. **Frontend:** Receives `timer-changed` event, refreshes active session state, shows "(paused)" label with idle reason.

**State Management:**

- Frontend: Svelte 5 `$state` reactive variables in store classes. No external state manager.
- Backend: SQLite database is the single source of truth. Tauri `State<Db>` provides shared access.
- Cross-system: Events (`tauri::emit`) notify frontend of database changes (e.g., "timer-changed" when tray pauses).

## Key Abstractions

**ActiveSessionInfo:**

- Purpose: Represents the currently-running timer with derived display data.
- Examples: `src-tauri/src/timer/mod.rs:18`, `src/lib/stores/timer.svelte.ts:3`.
- Pattern: Serialized struct passed over IPC; frontend unpacks and syncs local store.

**TaskRecord:**

- Purpose: Complete task metadata with computed aggregates (tracked_seconds, descendants via CTE).
- Examples: `src/lib/api.ts:68`, `src-tauri/src/commands/tasks.rs:6`.
- Pattern: Recursive `TASK_SELECT` query pulls task + sum of all descendant session time.

**Session:**

- Purpose: Low-level timer data (start_ts, end_ts, pause info, final_duration_seconds).
- Examples: `src-tauri/src/timer/mod.rs:7`.
- Pattern: Immutable once row is inserted; updates only set pause/end timestamps or final_duration.

**TimerError:**

- Purpose: Tagged enum for error communication across IPC boundary.
- Examples: `src-tauri/src/timer/mod.rs:28` (ActiveSessionConflict, NotFound, Other).
- Pattern: Frontend switches on `error.kind` to render context-specific UI (conflict modal, toast, etc.).

## Entry Points

**Frontend Entry Point:**

- Location: `src/app.html` (Svelte Kit's root HTML template).
- Triggers: Browser load (from Tauri webview).
- Responsibilities: Mounts SvelteKit app, loads initial layout and route.

**Backend Entry Point:**

- Location: `src-tauri/src/main.rs` (delegates to `lib.rs`).
- Triggers: Tauri app initialization.
- Responsibilities: Initializes Tauri builder, registers plugins, runs migrations, sets up event listeners, spawns background threads.

**Command Entry Points (IPC):**

- Location: `src-tauri/src/lib.rs:115–209` (invoke_handler macro).
- Triggers: Frontend calls `invoke(command_name, params)`.
- Responsibilities: Route to appropriate command handler (e.g., `start_timer` → `commands::timer::start_timer`).

**Subsystem Setup:**

- Location: `src-tauri/src/lib.rs:67–91` (setup closure).
- Triggers: After app builder runs, before window shows.
- Responsibilities: Open database, recover Todoist outbox, apply vibrancy (macOS), register tray menu, spawn background threads.

## Architectural Constraints

- **Threading:** Single Tauri runtime thread executes all command handlers sequentially. Background subsystems (idle, reminders, exam_countdown, weekly_review) spawn separate OS threads at startup. Tauri handle and database connection are Send + Sync.
- **Global state:** Single shared `Db(Mutex<Connection>)` managed by Tauri's state container. Tray state (`TrayState`) holds menu item references and a pending action queue. Desktop settings stored in `DesktopState` with Mutex.
- **Circular imports:** None detected. Modules are layered: commands depend on db and subsystems; subsystems depend on db; db depends only on rusqlite.
- **Database locking:** All database access acquires `db.0.lock()`. If lock is poisoned or contended, operation fails gracefully with error message. No deadlock prevention beyond Mutex; single connection is not re-entrant.
- **IPC serialization:** All command arguments and return values must impl `Serialize`/`Deserialize`. Enums use tag + struct format for variants (see `TimerError`).
- **Frontend store lifecycle:** Stores init in `onMount()` of layout and subscribe to events. Cleanup in `onDestroy()` to avoid memory leaks.

## Anti-Patterns

### Double-Counting Time in Analytics

**What happens:** Parent task's `tracked_seconds` includes all descendant session time; if query is not careful, subtracting child totals from parent results in negative values.

**Why it's wrong:** Breaks estimate calculations and makes schedule coverage math invalid.

**Do this instead:** Always use `tracked_seconds_direct` (sessions started exactly on this task, not children) for per-task analytics. Use the recursive CTE + SUM only when computing "total time invested in this feature and all subtasks."

Reference: `src-tauri/src/commands/tasks.rs:74` and comment on line 71.

### Unfinished Sessions at Startup

**What happens:** App crashes or restarts with an active session (end_ts IS NULL). Recovery modal shows with option to resume or discard. If user ignores modal and navigates, stale session remains in database.

**Why it's wrong:** Timer bar will show old elapsed time; user confusion on re-restart.

**Do this instead:** Always call `timerStore.refresh()` at mount and on window focus. Recovery modal is required on startup if session exists. See `src/routes/+layout.svelte:39` (recovery = info !== null on startup).

### Direct Todoist Token in Database

**What happens:** Some early code paths might log or store token in database instead of keychain.

**Why it's wrong:** Leaks secrets in backups and exports.

**Do this instead:** Always use OS keychain (Rust `keyring` crate). Database stores only a marker (presence flag). See `src-tauri/src/todoist/client.rs` for token retrieval pattern.

## Error Handling

**Strategy:** Result<T, E> throughout Rust. Frontend errors are mostly connection/IPC failures (render toast, ask user to retry). Conflict errors (timer clash) are expected, not errors—render modal.

**Patterns:**

- Tauri commands return `Result<T, String>` (String is error message for JSON serialization).
- Frontend async functions catch errors and set `timerStore.error` or display modal/toast.
- Database errors are logged to stderr; user sees generic "Couldn't save" message.
- Todoist sync failures populate outbox; user can retry from UI without manual intervention.

## Cross-Cutting Concerns

**Logging:**

- Frontend: `console.log/error` (visible in devtools, Tauri debug console in dev mode).
- Backend: `eprintln!()` to stderr (visible in terminal when running `tauri dev` or in system logs for bundled app).
- No centralized log file; app state captured via export/import feature.

**Validation:**

- Frontend: Input masks (date, time), min/max on number fields, required field checks before submit.
- Backend: Tauri command arguments validated on entry (e.g., idle threshold 0–120). Database constraints (foreign keys, not-null on required columns).
- Todoist mappings: Verify project_id exists before allowing map_todoist_project command.

**Authentication:**

- No user accounts; app assumes single-user desktop environment.
- Todoist token stored in OS keychain, retrieved on sync attempts.
- macOS: Uses IOHIDSystem for idle detection (requires no auth).

---

*Architecture analysis: 2026-09-20*
