# Project Research Summary

**Project:** TaskTimer — Longitudinal Trends/Insights Feature
**Domain:** Time-series analytics layer for a single-user local time-tracking desktop app (Tauri 2 + Svelte 5 + SQLite/WAL)
**Researched:** 2026-09-20
**Confidence:** MEDIUM-HIGH (architecture and pitfalls research is codebase-grounded/HIGH confidence; stack and features research is cross-checked web synthesis/MEDIUM confidence)

## Executive Summary

This milestone adds a "trends" layer on top of an already-shipping snapshot-analytics module: per-class study-time trends across weeks/months, a streak-history visualization (not just current count), and time-of-day patterns. The domain precedent is clear and consistent — Toggl, RescueTime, WakaTime, and the Obsidian habit-tracker ecosystem all converge on the same three primitives (period-over-period trend charts, a GitHub-style streak calendar heatmap, and an hour-of-day/day-of-week heatmap), so the three Active requirements in PROJECT.md map directly onto well-established table-stakes patterns rather than novel UX. The recommended approach is deliberately conservative: add `uPlot` (~48KB) as the one new frontend dependency for line/bar trend charts, skip a second charting library unless the streak heatmap genuinely needs interaction beyond a hand-rolled CSS grid, and build the trend queries as a new sibling Rust module (`commands/trends.rs`) that reuses the existing `DURATION_EXPR` and Monday-anchor week-boundary logic rather than forking them.

The single most important architectural decision is aggregation strategy: at this app's scale (one user, thousands not millions of session rows), on-the-fly `GROUP BY strftime(...)` queries are correct and materialized rollup tables would be premature complexity with real correctness risk (trigger drift, backfill-on-migration). Streak-history should extend the existing, already-tested Rust `BTreeSet<NaiveDate>` walk rather than reimplementing gaps-and-islands logic in SQL — two parallel streak implementations is a correctness risk, not just a style preference.

The primary risk is not feature scope but data-layer correctness bugs that are invisible until a user hits them months later: (1) wrapping the indexed `start_ts` column in `date()`/`strftime()` defeats the existing index and forces a full table scan on every trend query, worsening as the feature multiplies query volume; (2) reinventing week-bucketing with SQLite's `%W`/`%U` instead of reusing the codebase's existing Monday-anchor formula will produce a trend chart whose bars silently disagree with the "this week" snapshot card, worst exactly at semester/year boundaries; (3) plain `GROUP BY` silently omits buckets with zero sessions, which defeats the entire point of a "streak visualization" feature (gaps must render as visible zeros, not be collapsed out). All three are cheap to prevent up front and expensive to debug after the fact — they should be treated as launch-blocking, not polish.

## Key Findings

### Recommended Stack

STACK.md recommends `uPlot` (^1.6.32) as the sole new dependency for time-series line/area/bar trend charts — smallest and fastest canvas-based option (~48KB vs. Chart.js's ~254KB), sufficient for the narrow chart-type needs here (trend lines, time-of-day bars), and appropriate for a desktop app where startup snappiness matters even though bundle-size pressure is lower than a public web app. `LayerCake` is a conditional, deferred addition — only reach for it if the streak calendar heatmap needs real scale/zoom/tooltip interaction that a plain CSS grid + Svelte `{#each}` can't express; try the zero-dependency CSS-grid approach first. No date library (`date-fns`/`dayjs`/`luxon`) is needed since all bucketing happens in SQL and label formatting is a one-line `Intl.DateTimeFormat` call. On the backend, no new dependency is needed at all — `rusqlite` 0.40.2 (already in `Cargo.toml`) and the existing `strftime()`/`date()` bucketing idiom cover every requirement; this is a direct extension of `analytics.rs`'s established pattern, not a new abstraction.

**Core technologies:**
- `uPlot` ^1.6.32 — canvas-based time-series trend charts — smallest/fastest option that still ships real chart primitives (axes, cursor, tooltip); avoids Chart.js's 5x bundle tax for capability this feature doesn't need
- `rusqlite` 0.40.2 (existing) — time-bucketed SQL aggregation via `strftime()`/`date()` — already the codebase's idiomatic convention; no `date_trunc()` in SQLite, no reason to introduce a new abstraction
- `chrono` (existing) — Rust-side calendar-spine generation and streak-history walking — already used throughout `analytics.rs`/`timer/mod.rs`; needed because `rusqlite` is built without the `series` feature (no `generate_series()`)

### Expected Features

FEATURES.md cross-checked five mature products (Toggl Track, RescueTime, WakaTime, Forest, Obsidian habit/heatmap plugins) and found strong convergence: all three of PROJECT.md's Active requirements are table stakes, not novel bets, and the ecosystem also reveals two "free" derived stats (delta indicators, "most productive day") that fall out of the same data with near-zero extra backend cost.

**Must (table stakes):**
- Week-over-week / period-over-period comparison with a delta indicator — universal across every product reviewed; a query that diffs two snapshot periods plus a UI up/down indicator
- Per-class time breakdown trended over time — direct analog of Toggl's per-project pivot and WakaTime's per-language trend; same class breakdown already computed per period, repeated across N periods
- Streak calendar / contribution-graph heatmap (GitHub-style) — the single most consistent pattern across the entire habit-tracking ecosystem; now the expected way to show streak history, not just a current-streak number
- Time-of-day / hour-of-day heatmap (7x24 grid) — RescueTime and WakaTime both treat this as standard, not exotic; directly satisfies the third Active requirement
- Trend charts with a selectable time range (week/month/quarter/custom) — a fixed-window trend view feels rigid next to competitors
- "Most productive day/time" surfacing — a MAX() over data already needed for the heatmap; near-zero marginal cost once time-of-day data exists

**Should (differentiators, TaskTimer-specific, worth flagging for roadmap discussion but not required this milestone):**
- Cross-class time-of-day overlay ("which class do I study at night vs. morning") — no reviewed competitor has a "class" concept to combine with time-of-day
- Streak personal-best banner and streak-break annotations — cheap motivational payoff, computable from the same heatmap data
- Estimate-vs-actual trend (planned vs. tracked time per class over weeks) — unique to TaskTimer's existing estimate data, no reviewed competitor offers this
- Pomodoro cycle-completion trend — unique to Pomodoro-native tools, not offered by Toggl/RescueTime

**Defer (v2+ / explicit anti-features per PROJECT.md):**
- Social/team comparison, leaderboards, shared dashboards — out of scope, single-user local app
- Cloud-synced email digests or push notifications — no cloud/remote delivery mechanism in TaskTimer; render an in-app summary screen instead if wanted later
- Grade/outcome correlation and workload forecasting — explicitly deferred in PROJECT.md
- Gamification (trees, XP, badges) beyond simple streak counts — disproportionate scope for a visibility-focused feature
- Deadline-annotated trend chart (exam markers on trend lines) — genuinely valuable and in-bounds (doesn't touch grades), but adds chart-annotation complexity; reasonable later-phase candidate, not core to this milestone

### Architecture Approach

ARCHITECTURE.md's recommendation is directly grounded in the existing codebase (read `analytics.rs`, `tasks.rs`, `week.rs`, migrations, and the current `/analytics` route before proposing anything): add a new sibling Rust module `commands/trends.rs` next to `analytics.rs`, sharing (not duplicating) its `DURATION_EXPR` constant and Monday-anchor week-boundary formula; use on-the-fly `GROUP BY strftime()` aggregation with two new narrow indexes rather than materialized rollup tables; and add a new nested SvelteKit route `src/routes/analytics/trends/+page.svelte` (matching the codebase's existing folder-per-view convention) rather than cramming a fourth tab into the already ~300-line snapshot page.

**Major components:**
1. `commands/trends.rs` (new) — time-bucketed series queries: weekly/monthly per-class trend, streak history (bucket-by-bucket), time-of-day pattern; thin `#[tauri::command]` wrappers over plain `fn`s, matching `analytics.rs`'s existing pattern exactly
2. `routes/analytics/trends/+page.svelte` (new) — trend charts UI, reuses existing `$lib/api.ts` and `$lib/format.ts`; separate route because trend data has a fundamentally different shape (arrays of buckets) and re-fetch cadence than the existing today/week/month snapshot union
3. New composite/partial index (`idx_time_sessions_start_ts_task_id`, migration `0015_trend_indexes.sql`) — matches the existing partial-index convention (`WHERE end_ts IS NOT NULL`) and supports the "sessions in range, joined class, grouped by bucket" query shape all new trend queries will share
4. Rust-side calendar-spine + streak-history logic (extends existing `tracked_days()`/`BTreeSet<NaiveDate>` walk) — keeps gap-filling and streak semantics as a single source of truth rather than introducing a parallel SQL-side implementation

**Critical guardrail baked into the architecture itself:** every new trend query must aggregate `time_sessions` directly (never sum the recursive `tasks.tracked_seconds` field across a time bucket, which would double-count any subtask whose own session falls in the same bucket as its parent's).

### Critical Pitfalls

PITFALLS.md is grounded directly in the repo's own schema, migrations, and query code (HIGH confidence for codebase-specific findings), cross-checked against general SQLite/timestamp ecosystem pitfalls (MEDIUM confidence) for the rest.

1. **Non-sargable date predicates defeat the existing index** — any new query that wraps the indexed `start_ts` column in `date()`/`strftime()` in a WHERE/GROUP BY forces a full table scan every call. Prevention: add a denormalized `local_date TEXT` column populated at session-end time (preferred over N separate expression indexes, since sessions are also edited/pause-adjusted) and index that column normally; verify with `EXPLAIN QUERY PLAN` showing `SEARCH ... USING INDEX`, not `SCAN`.
2. **Reinvented week-bucketing disagrees with the existing "this week" definition** — SQLite's `%W`/`%U` are not ISO week numbering and don't match `get_analytics_week`'s existing Monday-anchor formula. Using them for the new trend query produces a chart whose last bar silently disagrees with the snapshot card, worst at semester/year boundaries. Prevention: extract the existing Monday-anchor arithmetic into one shared SQL fragment or Rust helper; single source of truth for both views.
3. **Empty buckets silently disappear instead of rendering as zero** — plain `GROUP BY` only emits rows for buckets with at least one session; a week off (exam break, illness) collapses into nothing rather than a visible dip, which directly undermines the point of a streak/gap visualization. Prevention: generate the full calendar spine in Rust (`chrono`, already a dependency) and left-merge SQL results onto it, defaulting missing buckets to zero — mirrors the existing `BTreeSet<NaiveDate>` streak-walk pattern.
4. **Local-day bucketing is recomputed against the *current* system timezone, not the timezone at record time** — because `'localtime'` is applied fresh at every query, a timezone change (travel, clock fix) can retroactively shift which calendar day/streak/week a historical session lands in. Prevention: same fix as Pitfall 1 (denormalized `local_date` column, populated once at session-end using the timezone active then) — a free side effect of the index fix.
5. **Duplicate streak-history logic in SQL vs. the existing tested Rust implementation** — reaching for SQL window-function gaps-and-islands for "streak over time" when the codebase already has a correct, tested `BTreeSet<NaiveDate>` walk creates two implementations of "what counts as a streak" that will eventually disagree at an edge case. Prevention: extend the existing Rust walk to compute streak-length-as-of-each-day rather than introducing a parallel SQL implementation.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Data-Layer Foundation (indexing, timezone denormalization, shared helpers)
**Rationale:** Every subsequent trend feature depends on this being right; retrofitting a denormalized `local_date` column or extracting the shared week-boundary helper *after* three sets of trend queries already exist is expensive rework. This is also where the three Critical pitfalls (non-sargable predicates, reinvented week logic, current-timezone recomputation) must be closed before they get baked into multiple query call sites.
**Delivers:** New migration adding `local_date` (and optionally `local_year_month`) column + index on `time_sessions`; a promoted/shared `DURATION_EXPR` and Monday-anchor week-boundary helper usable from both `analytics.rs` and the new `trends.rs`; the new sibling module skeleton (`commands/trends.rs`) with thin-command/shared-query-function pattern established.
**Features from FEATURES.md:** None directly user-facing yet — this is the enabling layer for all three Active requirements.
**Pitfalls avoided:** Critical #1 (non-sargable predicates), Critical #2 (reinvented week bucketing), Critical #4 (timezone recomputation).

### Phase 2: Per-Class Trend Charts (weekly/monthly, week-over-week comparison)
**Rationale:** Lowest complexity of the three Active requirements — a direct extension of existing snapshot/class-breakdown queries repeated across N periods, using the Phase 1 foundation. Delivers visible value fastest and validates the `uPlot` integration before the more novel heatmap work.
**Delivers:** `weekly_class_trend`/`monthly_class_trend` queries; period-over-period delta indicator; range selector (week/month/quarter/custom) reusing existing snapshot period logic; first `uPlot` line/bar chart in the new `analytics/trends` route.
**Features from FEATURES.md:** Week-over-week comparison, per-class trended breakdown, selectable time range, "most productive day" surfacing (near-zero marginal cost once per-day data flows through this path).
**Pitfalls avoided:** Critical #3 (empty buckets silently dropped) — must be solved here since it's the first place a calendar spine is needed; Moderate #2 (unbounded full-history scans) — bound queries by explicit date range from the start.

### Phase 3: Streak Calendar Heatmap and Streak History
**Rationale:** Depends on the calendar-spine/zero-filling pattern established in Phase 2, and must reuse (not reimplement) the existing tested `current_streak`/`longest_streak` Rust logic per Minor Pitfall #2. This is also the feature most likely to need the hand-rolled-CSS-grid-vs-LayerCake decision, best made once real day-cell data shape is in hand.
**Delivers:** Day-level streak history data (extends `tracked_days()` walk to compute streak-length-as-of-each-day); GitHub-style calendar heatmap component; streak personal-best stat.
**Features from FEATURES.md:** Streak calendar/contribution-graph heatmap, streak history (not just current count), streak personal-best (differentiator, cheap once base data exists).
**Pitfalls avoided:** Minor #2 (parallel SQL gaps-and-islands vs. existing Rust logic) — extend, don't reimplement; Moderate #2 (unbounded per-class streak scans).

### Phase 4: Time-of-Day Pattern Heatmap
**Rationale:** Independent data dimension (hour-of-day bucketing) not needed by Phases 2-3, so it can come last without blocking the other two Active requirements. Also the phase with the most novel/unaddressed pitfall (DST hour distortion), best handled once the team has already built confidence with the Phase 1 timezone-denormalization pattern.
**Delivers:** 7x24 day-of-week x hour-of-day heatmap, aggregated across all history or a rolling window; optional cross-class filter/overlay (differentiator, low incremental cost once base heatmap exists).
**Features from FEATURES.md:** Time-of-day/hour-of-day heatmap (table stake), cross-class time-of-day overlay (differentiator, consider bundling if time allows).
**Pitfalls avoided:** Moderate #1 (DST transitions distort hour buckets) — accept as a known, documented cosmetic tradeoff rather than over-engineering; bucket on UTC hour if precision is ever needed.

### Phase Ordering Rationale

- Phase 1 must come first because it closes the three Critical pitfalls before they can be baked into multiple query call sites — retrofitting later is strictly more expensive than building it first.
- Phases 2 and 3 are ordered by complexity and dependency: per-class trends are the most direct extension of existing snapshot code (validate the charting stack early), while streak history depends on the calendar-spine/zero-filling pattern that Phase 2 must solve anyway.
- Phase 4 is independent (a different bucketing dimension — hour, not day/week) and is ordered last because it's the only phase touching a genuinely new pitfall (DST) not encountered elsewhere, and benefits from the timezone discipline established earlier.
- The differentiator features (estimate-vs-actual, Pomodoro completion trend, deadline annotations) are deliberately excluded from this phase structure — FEATURES.md recommends deferring them past this milestone; they're candidates for a later milestone, not blockers to declaring the three Active requirements complete.

## Research Flags

Phases likely needing deeper research during planning:
- **Phase 3 (Streak Calendar Heatmap):** the uPlot-vs-LayerCake-vs-hand-rolled-CSS-grid decision is deliberately left open (MEDIUM confidence in STACK.md) and should be resolved with a quick prototype against real day-cell data shape before committing to a library.
- **Phase 1 (Data-Layer Foundation):** the denormalized `local_date` column approach is the STACK/PITFALLS recommendation, but the migration needs to handle backfilling existing rows correctly (test-fixture date-format drift risk, per Moderate Pitfall #3) — worth a focused pass during phase planning.

Phases with well-documented patterns (safe to skip research-phase):
- **Phase 2 (Per-Class Trend Charts):** direct extension of `analytics.rs`'s already-proven `strftime()`/`DURATION_EXPR` pattern; architecture and pitfalls are both HIGH confidence, codebase-grounded.
- **Phase 4 (Time-of-Day Heatmap):** same SQL bucketing idiom as Phase 2, only the bucket expression changes (hour instead of day/week); DST handling is an accepted, documented tradeoff, not an open question.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Cross-checked web benchmarks (uPlot vs. Chart.js/D3/ECharts) plus direct npm registry version checks; the SQLite bucketing recommendation is HIGH (verified directly against this repo's own `analytics.rs`, not web research) |
| Features | MEDIUM | Cross-checked across 5 independent mature products (Toggl, RescueTime, WakaTime, Forest, Obsidian plugins); every table-stake claim corroborated by at least 2 of the 5 |
| Architecture | HIGH | Grounded in direct source inspection of this repo's actual `analytics.rs`, `tasks.rs`, `week.rs`, migrations, and current `/analytics` route — not general web claims |
| Pitfalls | HIGH (codebase-specific) / MEDIUM (general ecosystem) | The four Critical pitfalls are read directly from this repo's schema/migrations/queries; DST and rollup-timing guidance is general web-search synthesis |

**Overall confidence:** MEDIUM-HIGH — the "what to build" question (features) rests on solid but external cross-checked research; the "how to build it correctly in this codebase" question (architecture, pitfalls) is unusually strong because it's grounded in direct inspection of the actual repo rather than general best practices.

## Gaps to Address

- **uPlot vs. LayerCake vs. hand-rolled CSS grid for the streak heatmap:** STACK.md explicitly flags this as MEDIUM confidence and recommends trying the zero-dependency approach first. Resolve during Phase 3 planning with a quick spike, not upfront.
- **ISO week vs. existing Monday-anchor week semantics at year boundaries:** ARCHITECTURE.md notes SQLite's `%W` disagrees with true ISO week numbering and with the codebase's own Monday-anchor formula; the phase-ordering above and the shared-helper extraction in Phase 1 are the mitigation, but the exact year-boundary edge case should get explicit test coverage during Phase 1/2 planning.
- **Denormalized `local_date` backfill strategy:** the recommended fix for Critical Pitfalls #1 and #4 requires a migration that populates `local_date` for all existing rows; the format-drift risk between `datetime('now')`-inserted rows and date-only test fixtures (Moderate Pitfall #3) means this backfill needs explicit handling of both formats, not an afterthought.
- **Data volume assumption:** the on-the-fly-aggregation-over-rollup-tables recommendation is LOW-MEDIUM confidence (general web synthesis, not benchmarked against this repo's actual data) — fine at "thousands of rows" scale per current PROJECT.md context, but should be re-evaluated if a future milestone pushes into tens of thousands of sessions per user.

## Sources

### Primary (HIGH confidence)
- Direct source inspection: `src-tauri/src/commands/analytics.rs`, `commands/tasks.rs`, `commands/week.rs`, `commands/mod.rs`, `db/mod.rs`, `db/migrations/0001_init.sql` through `0014_task_dependencies.sql`, `src-tauri/src/timer/mod.rs`, `src/routes/analytics/+page.svelte`, `src/lib/api.ts`, `src-tauri/Cargo.toml`, `.planning/codebase/CONCERNS.md`, `.planning/PROJECT.md`
- npm registry version checks for `uplot` (1.6.32), `layercake` (11.0.0), `chart.js` (4.5.1) as of 2026-09-20

### Secondary (MEDIUM confidence)
- Toggl Track features/reporting docs, RescueTime dashboard/productivity-report help articles, WakaTime API docs and dashboards, Forest official site, Obsidian Heatmap Tracker / Habit Tracker Pro plugin repos — cross-checked across 5 independent products for FEATURES.md
- uPlot GitHub benchmark data, LayerCake GitHub discussion, SciChart/LightningChart/Luzmo/Embeddable 2026 comparison pieces — cross-checked for STACK.md
- SQLite official date/time function docs and forum threads on `%W`/`%U`/`%V` week-numbering semantics — used for both ARCHITECTURE.md and PITFALLS.md

### Tertiary (LOW confidence)
- General web sources on rollup-vs-on-the-fly aggregation crossover points, DST-and-streak-engineering guides, and SQL gaps-and-islands technique references — used only as fallback/context; primary recommendations in all cases defer to the existing codebase's established patterns over these general claims

---
*Research completed: 2026-09-20*
*Ready for roadmap: yes*
