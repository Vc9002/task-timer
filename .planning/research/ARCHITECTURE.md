# Architecture Patterns

**Domain:** Longitudinal trend analytics layer on top of an existing Tauri 2 + Svelte + SQLite (WAL) snapshot-analytics module
**Researched:** 2026-09-20
**Overall confidence:** HIGH for codebase-grounded recommendations (read directly from source); LOW/MEDIUM for general SQLite/Tauri ecosystem practices (unverified web search, single-provider)

## Recommended Architecture

**New Rust module `src-tauri/src/commands/trends.rs`, sibling to `analytics.rs`, sharing its SQL primitives. On-the-fly `GROUP BY strftime()` bucketing (no materialized views) with two new narrow indexes. New Svelte route `src/routes/analytics/trends/+page.svelte` nested under the existing `/analytics` route, linked as a new tab.**

```
src-tauri/src/commands/
  analytics.rs   <- snapshot stats (today/week/month, estimate accuracy, streaks) — UNCHANGED
  trends.rs      <- NEW: time-bucketed series (weekly/monthly per-class hours,
                    streak history, time-of-day heatmap)

src/routes/analytics/
  +page.svelte           <- existing snapshot dashboard — UNCHANGED, add nav link
  trends/+page.svelte    <- NEW: trend charts, reuses $lib/api.ts + $lib/format.ts

src/lib/api.ts
  // ---------- Analytics ----------   <- existing section, unchanged
  // ---------- Trends ----------      <- NEW section, same file (see rationale below)
```

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| `commands/analytics.rs` | Point-in-time / range snapshots (today, this week, this month, weekly review, streak *counts*) | SQLite via `Db` state; called by `/analytics` page |
| `commands/trends.rs` (new) | Time-bucketed series: per-class hours by week/month, streak *history* (bucket-by-bucket, not just current/longest), time-of-day distribution | SQLite via `Db` state; called by `/analytics/trends` page; reuses `DURATION_EXPR` and query shape from `analytics.rs` |
| `db/mod.rs` | Connection setup, migrations, WAL/pragma config | Owns the `Db` `Mutex<Connection>` both modules lock |
| `src/lib/api.ts` | Typed `invoke()` wrappers + response interfaces | Single file for all Tauri IPC bindings (existing convention — every domain, not just analytics, lives here: tasks, week, planner, todoist, etc.) |
| `routes/analytics/+page.svelte` | Snapshot dashboard (today/week/month tabs) | `$lib/api.ts` |
| `routes/analytics/trends/+page.svelte` (new) | Longitudinal charts: per-class trend lines, streak calendar/history, time-of-day heatmap | `$lib/api.ts` |

### Data Flow

```
time_sessions (raw rows, WAL)
      │
      │  DURATION_EXPR (shared const, computed duration per session)
      ▼
commands/trends.rs query functions
  ├─ weekly_class_trend(conn, weeks_back)   → GROUP BY class, strftime('%Y-%W', start_ts)
  ├─ monthly_class_trend(conn, months_back) → GROUP BY class, strftime('%Y-%m', start_ts)
  ├─ streak_history(conn, class_id?)        → reuses tracked_days() from analytics.rs,
  │                                            window-function gap/island grouping
  └─ time_of_day_pattern(conn, range)       → GROUP BY class, strftime('%H', start_ts, 'localtime')
      │
      ▼
#[tauri::command] thin wrappers (lock conn, call fn, map_err(to_string))
      │  invoke("get_weekly_class_trend", {...})
      ▼
src/lib/api.ts  →  routes/analytics/trends/+page.svelte  →  chart components
```

## Why `trends.rs` as a sibling module, not inside `analytics.rs`

`analytics.rs` is already 960 lines covering four distinct concerns (range summaries, estimate accuracy, weekly review, streak *counts*). Codebase precedent already splits by concern rather than by "everything analytics-ish": `commands/week.rs` is separate from `commands/planner.rs` and `commands/today.rs` even though all three query `tasks`/`time_sessions`. Adding trend/time-series queries to `analytics.rs` would push it past 1,200+ lines and mix two different query shapes (point-in-time snapshot vs. multi-bucket series) in one file — the file's own doc comments (`DURATION_EXPR`, `due_date_expr`) already show the author cares about keeping shared primitives visible and reused, which a new sibling module preserves without duplication.

**Do not duplicate `DURATION_EXPR` or the direct/recursive distinction.** `analytics.rs` defines `DURATION_EXPR` as a private `const &str` — it is not `pub(crate)`. Two options, in order of preference:
1. Promote `DURATION_EXPR` (and the `due_date_expr` helper) to `pub(crate)` in `analytics.rs` and `use crate::commands::analytics::DURATION_EXPR;` from `trends.rs`. Lowest-risk, smallest diff, keeps single source of truth.
2. If more than one more module ends up needing it, extract both into a new `commands/query_fragments.rs` (or `commands/shared.rs`) and have `analytics.rs` and `trends.rs` both depend on it. Don't do this preemptively — YAGNI until a third consumer appears.

**Direct vs. recursive duration — this applies to every new trend query.** `tasks.rs`'s `TASK_SELECT` computes two different sums: a recursive-CTE `tracked_seconds` (walks `parent_task_id` children, used for "how much time did this task-and-its-subtasks take") and `tracked_seconds_direct` (`SUM(...) WHERE task_id = t.id`, no recursion). `analytics.rs`'s `DURATION_EXPR` + all of its `GROUP BY class`/`GROUP BY date` aggregates are effectively the "direct" flavor already, because they sum straight from `time_sessions` without walking the task tree — that's correct and is why weekly/monthly/streak numbers don't double-count parent+child time today. **Every new trend query must follow the same shape:** aggregate `time_sessions` directly (optionally joined to `tasks`/`classes` for grouping), never sum `tasks.tracked_seconds` (the recursive one) across a time bucket — that would double-count any task that has subtasks whose own sessions also fall in the bucket.

## SQLite time-bucketing at this app's scale

**Verdict: on-the-fly aggregation, not materialized views.** This is a single-user desktop app; "thousands of sessions" is 3–4 orders of magnitude below where SQLite rollup tables start paying for their complexity (web sources put the crossover around 1M+ raw rows before on-the-fly `GROUP BY` becomes a measurable bottleneck, and rollups are usually justified past ~10M). At thousands of rows, a `GROUP BY strftime(...)` query with an index on the timestamp column runs in single-digit milliseconds. Introducing a rollup table (and the write-time trigger or periodic-job logic to keep it in sync) would add real complexity — cache invalidation, backfill-on-migration, drift risk — for zero perceptible benefit at this data volume. *(Confidence: LOW-MEDIUM — general web-search synthesis, not benchmarked against this repo's actual data; if a future milestone reaches tens of thousands of sessions per user, re-evaluate.)*

**Bucketing expressions to use** (SQLite has no `date_trunc`; `strftime` is the idiomatic substitute, matching the existing codebase's use of `strftime('%Y-%m', ...)` in `get_analytics_month` and `strftime('%w', ...)` in `get_analytics_week`):
- Week bucket: `strftime('%Y-%W', ts.start_ts, 'localtime')` (ISO-ish week-of-year; note SQLite's `%W` is Monday-start week-of-year, not ISO week — acceptable here since the existing week logic already computes Monday-start ranges manually in `get_analytics_week`; for cross-year continuity in a trend chart, prefer bucketing by an explicit `date(ts.start_ts,'localtime','weekday 0','-6 days')` "week start date" string instead of `%Y-%W`, so buckets sort and merge correctly across year boundaries — `%W` resets oddly at year edges).
- Month bucket: `strftime('%Y-%m', ts.start_ts, 'localtime')` — already used verbatim in `get_analytics_month`, reuse directly.
- Time-of-day bucket: `strftime('%H', ts.start_ts, 'localtime')` (hour-of-day, 00–23), optionally collapsed into coarser bands (morning/afternoon/evening/late-night) in Rust after grouping, not in SQL — keeps the SQL simple and the labeling logic testable.

**Indexes to add** (new migration `0015_trend_indexes.sql`): the existing schema has `idx_time_sessions_task_id` and `idx_time_sessions_start_ts` (added in `0004_week_planning.sql`) but no composite covering the join+filter+group pattern trend queries will use. Add:
```sql
-- Speeds "sessions in date range, joined to class, grouped by bucket" —
-- the shape every new trend query shares.
CREATE INDEX idx_time_sessions_start_ts_task_id
  ON time_sessions(start_ts, task_id) WHERE end_ts IS NOT NULL;
```
A partial index (`WHERE end_ts IS NOT NULL`) matches the existing `one_active_session` partial-index convention already in `0001_init.sql`, and every trend/analytics query already filters on `end_ts IS NOT NULL`, so the partial index stays small and fully covers the hot path. At thousands-of-rows scale this is a nice-to-have, not a blocker — ship without it first if time-boxed, add it if `EXPLAIN QUERY PLAN` shows a full table scan.

**Streaks specifically:** `analytics.rs` already computes current/longest streak by loading all distinct tracked days into a `BTreeSet<NaiveDate>` and walking it in Rust (`tracked_days()` + the streak-walk functions near line 823). This is the right pattern to extend for "streak *history*" (e.g., a calendar heatmap or a list of every streak this semester) — reuse `tracked_days()` as-is and do the gap/island grouping in Rust, not SQL. Rust-side iteration over a `BTreeSet` of a few hundred dates is trivial and keeps the existing, already-tested logic (the file's own `#[cfg(test)]` block covers exactly this) as the single source of truth, rather than reimplementing gaps-and-islands as a second parallel `LAG()`/`ROW_NUMBER()` SQL query. Only reach for SQL window functions if a *per-class-per-week* streak matrix is needed (i.e., grouping is itself multi-dimensional) — SQLite has supported window functions since 3.25 (rusqlite's bundled SQLite is far newer), so `ROW_NUMBER() OVER (PARTITION BY class_id ORDER BY date) ` / date-minus-row-number gaps-and-islands is available if needed later. *(Confidence: LOW — general SQL pattern from web search, not benchmarked in this codebase.)*

## Svelte route structure

**Recommendation: nest under the existing `/analytics` route as `/analytics/trends`, not a new top-level route, and not more tabs crammed into the existing 300-line `+page.svelte`.**

Current state: `routes/analytics/+page.svelte` is a single file (~300 lines incl. inline `<style>`) with client-side tab state (`range: "today" | "week" | "month"`) that re-fetches on tab change. It has no sub-routes and no shared layout file (`+layout.svelte`) today.

Reasons for a nested route over adding a fourth tab value to the existing page:
1. **Different data shape, different re-fetch cadence.** Today/week/month are three variants of the *same* snapshot query (`RangeSummary`, `EstimateAnalytics`, `WeeklyReview`, `Streaks`), re-fetched together in one `load()`. Trends are a fundamentally different shape (arrays of buckets for charting) with their own params (weeks-back, months-back, per-class filter). Folding them into the same `range` union and the same `load()` function would force awkward branching (`if (range === "trends") { ... completely different fetch and completely different render ... }`) inside a file that's already dense single-line markup.
2. **File-size trajectory.** The existing page is already ~300 lines handling 4 sub-features (hero stats, streaks, estimate accuracy, weekly review). Trend charts (likely 3 chart types: per-class line/bar trend, streak calendar, time-of-day heatmap) are their own visual complexity — a separate route file keeps both files navigable and lets a future contributor find "where do I edit the streak calendar" without scanning past unrelated snapshot markup.
3. **SvelteKit convention match.** The project already uses nested route folders for sibling concerns elsewhere (`routes/week/`, `routes/semester/`, `routes/calendar/` are all top-level siblings rather than tabs-in-one-page), so `analytics/trends/` as a child route is consistent with how this codebase already separates "different views over related data" — it's a folder-per-view app, not a tabs-in-one-page app, everywhere except inside `/analytics` itself (where today/week/month are tabs because they share one query shape).

Concretely:
- Add `src/routes/analytics/trends/+page.svelte` (new file).
- In `routes/analytics/+page.svelte`, add a plain link/nav item to `/analytics/trends` next to (or replacing the flat button-row styling of) the existing `.tabs` element — reuse the same `.tabs` CSS class for visual consistency, just as an `<a href="/analytics/trends">` instead of a `<button onclick>`.
- No `+layout.svelte` is needed unless a persistent nav header between the two pages is wanted later; skip it for now (YAGNI — nothing in the codebase uses layouts for `/analytics` today).
- `$lib/api.ts` stays a single file (existing convention: every domain — tasks, week, planner, todoist, exams — lives in this one file under a `// ---------- X ----------` comment banner). Add a new `// ---------- Trends ----------` section rather than splitting into `api/trends.ts`; splitting would break the established one-file-per-frontend convention for a marginal win, and the file's existing section-banner structure already makes a new domain easy to locate.

## Patterns to Follow

### Pattern 1: Thin command, shared query function
**What:** Each `#[tauri::command]` in `trends.rs` should be a thin wrapper — lock the connection, call a plain (non-`#[tauri::command]`) `fn` that does the actual query and returns a `rusqlite::Result<T>`, then `.map_err(|e| e.to_string())`. This is exactly the existing pattern in `analytics.rs` (`range_summary()`, `session_stats()`, `block_summary()`, `workload_days()` are all plain functions called by one or more `#[tauri::command]` functions).
**When:** Every new trend query.
**Example:**
```rust
fn weekly_class_trend(
    conn: &rusqlite::Connection,
    weeks_back: i64,
) -> rusqlite::Result<Vec<WeeklyClassBucket>> {
    let sql = format!(
        "SELECT date(ts.start_ts,'localtime','weekday 0','-6 days') AS week_start,
                c.id, c.course_code, COALESCE(SUM({DURATION_EXPR}), 0)
         FROM time_sessions ts
         JOIN tasks t ON t.id = ts.task_id
         JOIN classes c ON c.id = t.class_id
         WHERE ts.end_ts IS NOT NULL
           AND date(ts.start_ts,'localtime') >= date('now','localtime','-{} days')
         GROUP BY week_start, c.id, c.course_code
         ORDER BY week_start",
        weeks_back * 7
    );
    // ...
}

#[tauri::command]
pub fn get_weekly_class_trend(db: State<Db>, weeks_back: i64) -> Result<Vec<WeeklyClassBucket>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    weekly_class_trend(&conn, weeks_back).map_err(|e| e.to_string())
}
```

### Pattern 2: Reuse `DURATION_EXPR`, never re-derive session duration
**What:** Session duration (`COALESCE(final_duration_seconds, computed-from-timestamps-minus-pauses)`) is a 6-line expression with a non-obvious edge case (open/crashed sessions contribute 0, not NULL). It is defined once in `analytics.rs`.
**When:** Any query touching `time_sessions.start_ts`/`end_ts`.
**Why it matters:** re-deriving this by hand risks silently dropping the `accumulated_pause_seconds` subtraction or the `final_duration_seconds` override, producing trend numbers that don't match the snapshot numbers on the same page for the same date range — a confusing, hard-to-spot bug (users comparing "this week" snapshot total against a trend chart's "this week" bucket and seeing them disagree).

## Anti-Patterns to Avoid

### Anti-Pattern 1: Aggregating `tasks.tracked_seconds` (recursive) across a time bucket
**What:** Grouping/summing the recursive `tracked_seconds` field (from `tasks.rs`'s `TASK_SELECT`, which walks `parent_task_id` children) by week/month instead of aggregating raw `time_sessions` rows directly.
**Why bad:** A parent task and its subtasks each contribute their own session time; the recursive sum on the parent already includes the children's time. Bucketing that recursive field again by date would double-count any subtask session whose timestamp falls in the same bucket as its parent's session — silently inflating weekly/monthly totals for any class using subtasks. This is precisely the failure mode the milestone brief calls out.
**Instead:** Aggregate `time_sessions` directly per bucket (the same approach `DURATION_EXPR` + `GROUP BY` already uses in `analytics.rs` for today/week/month), which sums each session exactly once regardless of the task hierarchy.

### Anti-Pattern 2: Materialized rollup tables / triggers at this scale
**What:** Adding `time_sessions_weekly_rollup` / `time_sessions_monthly_rollup` tables maintained by `AFTER INSERT/UPDATE` triggers on `time_sessions`.
**Why bad:** Real engineering cost (trigger correctness on edit/delete of a session, migration backfill, drift risk between raw and rollup on schema changes) for a workload — thousands of sessions for one user — where a plain indexed `GROUP BY` already returns in single-digit milliseconds. This is premature optimization for the stated scale.
**Instead:** On-the-fly `GROUP BY strftime(...)` with the composite partial index described above. Revisit only if a future milestone's data volume assumption changes by orders of magnitude.

### Anti-Pattern 3: A second copy of `DURATION_EXPR` in `trends.rs`
**What:** Pasting the `COALESCE(final_duration_seconds, ...)` expression into `trends.rs` as its own local `const` because `analytics.rs`'s version is private.
**Why bad:** Two copies of a duration-computation expression will drift the moment either one is edited for a bugfix (e.g., a future pause-tracking change) — this is the literal "duplicating logic" the milestone brief asks to avoid.
**Instead:** Promote the existing `const` to `pub(crate)` and `use` it from `trends.rs` (see "Why `trends.rs` as a sibling module" above).

## Scalability Considerations

| Concern | At current scale (1 user, thousands of sessions) | If it ever grows (10K+ sessions/user, still single-user desktop) | Notes |
|---------|------------|---------|-----|
| Time-bucket queries | Plain `GROUP BY strftime()`, indexed, sub-10ms | Still fine — index keeps it a range scan, not a table scan | Add the composite partial index proactively; it's cheap and matches existing partial-index convention |
| Streak history | Rust-side `BTreeSet<NaiveDate>` walk (existing pattern) | Still fine — a few thousand distinct dates is trivial in-memory | Don't move to SQL window functions unless per-class-per-week matrices are needed |
| Multi-year trend charts | N/A yet (app is new) | Bucket by explicit week-start date string, not `%Y-%W`, to avoid year-boundary bucket-merging bugs | Called out above; matters once >1 year of data exists |
| Connection contention | Single `Mutex<Connection>` (existing `Db` state), WAL mode already on | Same — WAL mode already supports concurrent readers; single-user desktop app has no concurrent-writer contention to worry about | No change needed; `db/mod.rs` already sets `PRAGMA journal_mode = WAL` |

## Sources

- Direct source inspection (HIGH confidence): `/Users/vincentc9002/task-timer/src-tauri/src/commands/analytics.rs`, `commands/tasks.rs`, `commands/week.rs`, `commands/mod.rs`, `db/mod.rs`, `db/migrations/0001_init.sql` through `0014_task_dependencies.sql`, `src/routes/analytics/+page.svelte`, `src/lib/api.ts`, `src-tauri/Cargo.toml` (rusqlite 0.40.2, bundled SQLite).
- [Time-Series Data in SQLite: Patterns Before Reaching for TimescaleDB](https://anethoth.com/time-series-data-in-sqlite/) — LOW confidence, general web search, rollup-vs-on-the-fly crossover point.
- [SQLite Analytics Dashboards and Reporting Pipelines](https://www.sqliteforum.com/p/designing-analytics-dashboards-powered) — LOW confidence, general web search.
- [How to GROUP BY Month and Year in SQLite? - GeeksforGeeks](https://www.geeksforgeeks.org/sqlite/how-to-group-by-month-and-year-in-sqlite/) — LOW confidence, `strftime()` bucketing syntax reference.
- [How to Reasonably Keep Your Tauri Commands Organized in Rust - DEV Community](https://dev.to/n3rd/how-to-reasonably-keep-your-tauri-commands-organized-in-rust-2gmo) — LOW confidence, general web search, command-module organization pattern.
- [Calling Rust from the Frontend | Tauri v2 official docs](https://v2.tauri.app/develop/calling-rust/) — MEDIUM confidence (official docs, via web search not Context7).
- [Gap and Island SQL – Advanced Query Pattern Explained](https://www.practicewindowfunctions.com/learn/gap_and_island) — LOW confidence, general web search, gaps-and-islands technique reference (used here only as a fallback note; primary recommendation is to keep the existing Rust-side streak logic).
