---
last_mapped_commit: 3628ccdef63f6dfb4e6d6b20ffce30c3d515cda3
last_mapped_at: 2026-09-20
---
# Codebase Concerns

**Analysis Date:** 2026-09-20

## Tech Debt

**SQL Injection Risk in Export/Restore:**

- Issue: Table names are interpolated directly into SQL queries using `format!()` instead of parameterized queries
- Files: `src-tauri/src/export.rs` (lines 80, 96)
- Impact: While currently the table names come from hardcoded schema restore operations, this is a security anti-pattern that could become dangerous if table names are ever derived from user input
- Fix approach: Use a whitelist of allowed table names with explicit matching instead of string interpolation

**Unsafe Unwrap in Production Code:**

- Issue: `external_id.unwrap()` at line 59 in `src-tauri/src/todoist/sync.rs` is safe due to prior `is_none()` check, but idiomatically should use `.expect()` with a message or handle via pattern matching
- Files: `src-tauri/src/todoist/sync.rs`
- Impact: Reduces code clarity and makes intent harder to read; if refactored, the safety contract could be violated
- Fix approach: Replace with `.expect("external_id must be Some after None check")` or eliminate the `is_none()` check and use `if let Some(external_id) = external_id` pattern

**Monolithic Command Handlers:**

- Issue: Several command modules contain 400+ lines of logic in single files, making them difficult to test and reason about
- Files: `src-tauri/src/commands/analytics.rs` (804 lines), `src-tauri/src/commands/planning.rs` (541 lines), `src-tauri/src/export.rs` (431 lines)
- Impact: Hard to isolate failures, difficult to reuse query logic, increases cognitive load during maintenance
- Fix approach: Extract repeated SQL queries into helper functions in `src-tauri/src/db/mod.rs`, split analytics into sub-modules by concern (session stats, class totals, estimates), move export logic to separate module

**String Cloning Overhead:**

- Issue: 254 clone/to_string() operations across command handlers, primarily via `to_string()` calls on dates, IDs, and JSON
- Files: All files in `src-tauri/src/commands/`
- Impact: Unnecessary allocations during request handling; minor performance overhead but adds up across many handlers
- Fix approach: Prefer `&str` parameters where possible, use `Cow<str>` for conditional ownership, leverage Rust's zero-copy patterns

## Known Bugs

**Export Preserves Database Structure but Schema Drift Silent:**

- Symptoms: `restore_backup()` in `src-tauri/src/export.rs` silently tolerates missing columns when importing old backups
- Files: `src-tauri/src/export.rs` (lines 75-102)
- Trigger: User exports with older version, new version adds columns to schema, import succeeds but new columns are NULL
- Workaround: New columns have NOT NULL DEFAULT values to ensure consistency; currently acceptable but fragile

**Todoist Completion State Not Fully Bidirectional:**

- Symptoms: Completing a task locally queues a Todoist API call, but task stays in TaskTimer as "completed" even if the API call fails later
- Files: `src-tauri/src/todoist/sync.rs` (lines 45-72), `src-tauri/src/commands/todoist.rs`
- Trigger: User completes Todoist task locally, network fails during outbox flush, Todoist still shows incomplete
- Workaround: Manual sync pulls the correct state from Todoist; retry logic exists but may take several syncs

**Notification Preferences Silent Fallback:**

- Symptoms: If `app_settings` JSON parsing fails, app silently defaults to enabled=false, 25% threshold without logging the error
- Files: `src-tauri/src/reminders.rs` (line 41)
- Trigger: Corrupted or manually edited `app_settings` row
- Workaround: Settings page re-saves valid JSON on any change

## Security Considerations

**Todoist Token Storage Relies on OS Keychain:**

- Risk: If OS keychain is compromised, token is exposed; if keychain unavailable (e.g., headless systems), token read fails silently
- Files: `src-tauri/src/todoist/mod.rs`
- Current mitigation: Keyring crate handles secure OS storage; token never written to disk or SQLite
- Recommendations: Log keychain access failures clearly, provide fallback mechanism or clear error message if token cannot be retrieved on startup

**Database Backup Export Excludes Secrets but Format Version Unversioned:**

- Risk: `backup()` in `src-tauri/src/export.rs` includes `format_version: 1` but no version negotiation on restore; future schema changes could break import silently
- Files: `src-tauri/src/export.rs` (line 44)
- Current mitigation: `restore_backup()` tolerates missing columns; future versions should version-gate restore logic
- Recommendations: Add explicit `format_version` check in restore, fail loudly if format is unsupported, document breaking changes in CHANGELOG

**No Rate Limiting on Frontend-to-Backend Commands:**

- Risk: Malicious frontend code (via extension or compromise) could spam heavy queries (analytics, exports) and crash the app
- Files: All command handlers in `src-tauri/src/commands/`, invoked via `#[tauri::command]`
- Current mitigation: Tauri's invoke bridge is single-threaded for frontend; SQLite mutex serializes backend work
- Recommendations: Add per-command rate limits or debounce in Tauri, reject too-frequent requests with clear error

## Performance Bottlenecks

**Analytics Queries Not Indexed:**

- Problem: `get_analytics_*` queries in `src-tauri/src/commands/analytics.rs` aggregate across `time_sessions` and `tasks` with complex joins, likely doing full table scans
- Files: `src-tauri/src/commands/analytics.rs` (lines 50-200 estimated, exact line numbers compressed)
- Cause: No indices on `task_id`, `class_id`, `start_ts` in `time_sessions` table; DATE filtering on `start_ts` requires full scan
- Improvement path: Add indices on `(task_id)`, `(class_id)`, `(start_ts)` in schema migration; profile queries with `.explain query plan` to confirm improvements

**Idle Detection Polling in Separate Thread:**

- Problem: `src-tauri/src/idle.rs` spawns a thread that sleeps 1 second and loops indefinitely, polling OS idle state even when app is backgrounded
- Files: `src-tauri/src/idle.rs` (line 87)
- Cause: Could wake CPU unnecessarily on battery-powered devices
- Improvement path: Use OS event notifications (Cocoa on macOS, Win32 events on Windows) instead of polling; leverage Tauri's window focus events to pause polling when unfocused

**Reminder Check Loop Uses Condvar but Wakes Too Frequently:**

- Problem: Reminders thread wakes up on every timer state change and re-evaluates thresholds; could produce multiple notifications per threshold if timer is paused/resumed
- Files: `src-tauri/src/reminders.rs` (lines 187-200 estimated)
- Cause: No debouncing between state changes; condvar notify_one() called on every timer transition
- Improvement path: Add a minimum quiet period (e.g., 1 second) before re-checking threshold, store `last_check_ts` to suppress duplicate notifications

## Fragile Areas

**Timer State Machine Crash Recovery:**

- Files: `src-tauri/src/timer/mod.rs`
- Why fragile: Recovery logic at startup reads `time_sessions` with `end_ts IS NULL` to find in-progress sessions; if a session is left with `pause_started_ts` set but no `end_ts`, restart will offer "Continue" even if the pause time is stale (e.g., from a month ago)
- Safe modification: When calculating elapsed time in `compute_elapsed_seconds()`, validate that `pause_started_ts` is recent (within 24 hours); if not, log a warning and treat as a fresh start
- Test coverage: Tests exist for crash recovery but don't cover multi-day pause scenarios (`src-tauri/src/timer/mod.rs` tests)

**Todoist Sync Gate Blocks All Access:**

- Files: `src-tauri/src/todoist/sync.rs` (line 10)
- Why fragile: Single static `SYNC_GATE` Mutex blocks both `sync_now()` and `queue_completion()`; if a user tries to complete a task while sync is running, they get "sync already running" error instead of queueing locally
- Safe modification: Separate gates for sync and outbox operations; sync blocks itself, but completion queueing proceeds independently
- Test coverage: No tests for concurrent completion + sync; add test that starts sync, then tries to complete a task from a different thread

**Export/Restore Atomicity Depends on SQLite WAL:**

- Files: `src-tauri/src/export.rs`
- Why fragile: `restore_backup()` relies on `unchecked_transaction()` and manual `PRAGMA foreign_keys=OFF` to reorder inserts; if process crashes mid-transaction or SQLite is corrupted, restore could leave DB in inconsistent state
- Safe modification: Add a `restore_in_progress` flag to `app_settings`, check it on startup, and offer to rollback failed restores (by reverting to pre-restore WAL snapshot if available)
- Test coverage: Existing test at line 342 covers success path; add test that simulates crash during restore

**Task Completion Logic Not Synchronized with Todoist:**

- Files: `src-tauri/src/commands/tasks.rs`, `src-tauri/src/todoist/sync.rs`
- Why fragile: Local completion updates task status immediately; Todoist completion is queued asynchronously; if user syncs before queue flushes, local completion overwrites queued remote completion
- Safe modification: On sync, reconcile local `status='completed'` with Todoist completion queue; if both exist, mark as synced; if only local exists, re-queue
- Test coverage: Tests at line 299+ in `todoist/sync.rs` cover idempotency but not local-first completion

## Scaling Limits

**Single SQLite Connection with Mutex:**

- Current capacity: Single in-process reader/writer; concurrent requests serialize at `Db(Mutex<Connection>)` lock
- Limit: If UI sends many analytics queries while sync is running, requests queue up behind sync's lock; user sees UI lag
- Scaling path: Switch to connection pool (e.g., `r2d2`) or use SQLite's WAL mode more aggressively to allow concurrent reads; move heavy queries to async background tasks

**Tauri Async Runtime Single-Threaded for Frontend Commands:**

- Current capacity: Frontend can queue one command at a time; Tauri's invoke serializes
- Limit: If analytics query takes 2 seconds, UI is blocked from accepting any new commands (clicks, key presses)
- Scaling path: Use `tauri::async_runtime::spawn_blocking()` more liberally for heavy queries, return progress updates via events instead of waiting for completion

**No Pagination for Large Result Sets:**

- Current capacity: Queries like `list_sessions_for_task()` return all sessions in one result
- Limit: Tasks with thousands of sessions (unlikely but possible over years of use) will OOM
- Scaling path: Add offset/limit parameters to list endpoints, implement cursor-based pagination in UI

## Dependencies at Risk

**`keyring` Crate OS Compatibility:**

- Risk: `keyring` v4.2.0 requires specific Tauri plugin versions and may have platform-specific issues; if upgrade needed, compatibility must be verified
- Impact: Todoist token storage fails, user cannot set up Todoist sync
- Migration plan: Fallback to secure storage in SQLite with app-level encryption (not ideal, but better than failure)

**`reqwest` Blocking Client in Tauri Async Runtime:**

- Risk: `reqwest::blocking::Client` used in `src-tauri/src/todoist/client.rs` can deadlock if spawned in Tauri's async executor without `spawn_blocking()`; current usage is safe (called from sync gate which runs in spawn_blocking), but fragile if refactored
- Impact: Sync hangs or panics
- Migration plan: Use `reqwest::Client` (non-blocking) with async/await throughout, or wrap all HTTP calls in spawn_blocking

## Missing Critical Features

**No Backup on Startup:**

- Problem: User's entire time-tracking database lives in one SQLite file; no automatic backup before migrations run
- Blocks: Safe rollback if schema upgrade goes wrong
- Workaround: Manual export via Settings → Export

**No Conflict Resolution for Todoist Updates:**

- Problem: If task is edited in Todoist and locally at the same time, sync overwrites one with the other based on timestamp; no merge or conflict prompt
- Blocks: Simultaneous multi-device editing
- Workaround: Sync frequently, avoid editing in both places

**No Data Validation on Import:**

- Problem: CSV import for tasks doesn't validate date formats or required fields; bad rows are silently skipped
- Blocks: Batch migration from other tools with mixed data quality
- Workaround: Clean CSV manually before import

## Test Coverage Gaps

**No Frontend Unit Tests:**

- What's not tested: Svelte component logic, state management, form validation, error UI
- Files: All files in `src/lib/components/`, `src/routes/`, `src/lib/stores/`
- Risk: UI bugs (incorrect date formatting, state sync issues, missing validation) ship to users
- Priority: High — frontend is user-facing and frequently changed

**No Integration Tests for Todoist Sync Edge Cases:**

- What's not tested: Sync with network failures mid-request, multiple syncs in rapid succession, task completion during sync, large projects (1000+ tasks)
- Files: `src-tauri/src/todoist/sync.rs`
- Risk: Edge cases cause silent sync failures or data corruption
- Priority: High — Todoist sync is critical path and hard to debug in production

**No E2E Tests for Timer Recovery:**

- What's not tested: Actual process crash + restart (simulated with `kill -9`), system sleep during session, clock adjustments (DST, NTP corrections)
- Files: `src-tauri/src/timer/mod.rs`
- Risk: Timer state becomes corrupted in real-world scenarios that unit tests don't cover
- Priority: High — timer is core feature and state must be reliable

**Export Backup Tests Only Cover Happy Path:**

- What's not tested: Restore with missing database columns, restore with extra columns in backup, restore mid-corruption
- Files: `src-tauri/src/export.rs`
- Risk: Backup/restore fails in production and user loses data
- Priority: Medium — can be added incrementally

**No Performance Benchmarks:**

- What's not tested: Analytics query time on 10K sessions, import time for 1K tasks, sync time for 100-project workspace
- Files: All command handlers
- Risk: Performance regressions go unnoticed until user complains
- Priority: Medium — establish baseline and gate new features on performance budget

---

*Concerns audit: 2026-09-20*
