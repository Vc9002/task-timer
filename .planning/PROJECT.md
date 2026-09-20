# TaskTimer

## What This Is

TaskTimer is a Tauri 2 + Svelte desktop app for student task and time tracking — classes, tasks, a pausable timer with Pomodoro mode, exam countdowns, weekly reviews, and bidirectional Todoist sync. It runs locally with a SQLite (WAL mode) backend. Built and used by a single student (Vincent) to manage coursework across multiple classes.

## Core Value

The timer and task data must always be trustworthy — accurate elapsed time, no silent data loss (backups/sync/export), no double-counted or dropped session time. Everything else (analytics, integrations) is secondary to this holding true.

## Requirements

### Validated

- ✓ Class, task, and subtask management with scheduling — existing
- ✓ Pausable/recoverable timer sessions (with idle auto-pause) — existing
- ✓ Pomodoro work/break cycles — existing
- ✓ Bidirectional Todoist sync — existing
- ✓ Daily/weekly/monthly analytics snapshots (totals, streaks, priority/class breakdowns, estimates) — existing
- ✓ Exam countdown and weekly review jobs — existing
- ✓ Backup/restore and export/import — existing (see Constraints — has known hardening gaps)

### Active

- [ ] User can view study-time trends per class across multiple weeks/months (not just single-period snapshots)
- [ ] User can see streak visualization over time (not just current streak count)
- [ ] User can see time-of-day patterns (when they actually study, not just how much)

### Out of Scope

- Grade/outcome correlation (connecting time invested to grades/exam scores) — explicitly deferred, not part of this scope
- Workload forecasting (deadline load vs. available time) — explicitly deferred, not part of this scope
- New integrations beyond Todoist (Canvas, Google Calendar, etc.) — explicitly deferred, not part of this scope

## Context

- Existing analytics backend (`src-tauri/src/commands/analytics.rs`, 804 lines) already computes daily/weekly/monthly stats, streaks, and priority/class breakdowns — these are point-in-time snapshots, not longitudinal trend data. The new analytics work should build on this backend rather than replace it.
- Data model note from codebase mapping: always use `tracked_seconds_direct` for per-task analytics (not the recursive-CTE child sum), or totals double-count. This applies to any new trend queries too.
- `.planning/codebase/CONCERNS.md` documents known hardening gaps (SQL injection risk via `format!()` table-name interpolation in export/restore, monolithic 400+ line command handlers, an unsafe `.unwrap()` in Todoist sync) — out of scope for this phase of work but tracked for a future hardening pass.

## Constraints

- **Tech stack**: Tauri 2 (Rust backend) + SvelteKit frontend + SQLite (WAL mode) — no new runtime dependencies unless clearly justified
- **Data integrity**: Any new analytics queries must use `tracked_seconds_direct` for per-task totals to avoid the double-counting bug documented in ARCHITECTURE.md
- **Local-only**: All data stays in the local SQLite file; no new cloud/remote analytics services

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Existing feature set treated as Validated, not re-scoped | App is already actively used daily; re-litigating shipped features wastes effort | — Pending |
| Trends/insights analytics chosen as next active scope (over hardening pass or new integrations) | User's stated priority; existing tech-debt items are tracked but not blocking | — Pending |
| Build on existing `analytics.rs` backend rather than a parallel system | Avoids duplicating the snapshot logic that's already correct and tested | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-09-20 after initialization*
