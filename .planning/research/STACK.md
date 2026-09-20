# Technology Stack

**Project:** TaskTimer — Longitudinal Trends/Insights Feature
**Researched:** 2026-09-20

## Context

This is a research addendum for a **subsequent milestone** on an existing Tauri 2 + Svelte 5 + SQLite (WAL) desktop app. The app currently has:
- No charting library in the codebase (`package.json` has zero chart/viz dependencies).
- An existing `src-tauri/src/commands/analytics.rs` (960 lines) that already computes daily/weekly/monthly snapshot stats using `rusqlite` (bundled SQLite) with hand-rolled `strftime`/`date()` bucketing and Rust-side streak computation over `Vec<NaiveDate>`.
- `adapter-static` (SvelteKit static/SPA build) — the frontend ships as static assets rendered client-side inside the Tauri webview, so any browser-compatible charting library works with zero SSR concerns.
- An `analytics/+page.svelte` route already scaffolded (currently a 26-line stub) as the natural home for trend visualizations.

The new feature adds: per-class study-time trends across weeks/months, streak visualization over time, and time-of-day patterns. This is inherently time-series visualization over a modest dataset (a single student's task/session history — thousands, not millions, of rows), rendered in a desktop app where bundle size still matters (Tauri apps are judged on startup snappiness) but is not as brutal a constraint as a public web app.

## Recommended Stack

### Charting Library
| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| **uPlot** | ^1.6.32 | Canvas-based time-series line/area charts (weekly/monthly trend lines, time-of-day heatmap-style bars) | Smallest and fastest option that still gives you a real "chart" out of the box (axes, legends, cursor/tooltip, zoom) rather than raw primitives. Official benchmark: ~48KB minified vs Chart.js's ~254KB — roughly 5x smaller — with lower render time and peak memory. For a local-first desktop app with no other charting in the codebase, this avoids paying a heavy bundle tax for a feature that's fundamentally "line charts over weekly/monthly buckets." MEDIUM confidence (cross-checked across multiple independent web sources plus uPlot's own published benchmark data). |
| **LayerCake** | ^11.0.0 (optional, for streak calendar/heatmap) | Svelte-native scales + SVG/Canvas composition layer for the one visualization uPlot doesn't naturally cover well — a GitHub-style streak calendar/heatmap | LayerCake is not a "chart library" with prebuilt chart types; it's a thin Svelte component framework that hands you scales and a coordinate system and lets you compose your own SVG/Canvas marks. Use it only for the streak-over-time calendar heatmap, which uPlot (built for continuous x/y series) doesn't model well. Skip it entirely if you're willing to hand-roll the streak heatmap as a plain CSS grid + Svelte `{#each}` (a completely valid low-dependency alternative — see Alternatives). MEDIUM confidence. |

**Recommendation: uPlot alone is likely sufficient.** Reach for LayerCake only if the streak heatmap needs real scale/interaction logic beyond what a CSS grid can express. Do not add both unless you hit that wall — two viz dependencies for one feature is more surface area than this app needs.

### Data-Aggregation Pattern (SQLite time-bucketing)
| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| `rusqlite` (bundled) | 0.40.2 (already in Cargo.toml) | Time-bucketed aggregate queries via `strftime()`/`date()` | No changes needed — SQLite has no `date_trunc()`, and the codebase already uses the idiomatic workaround (`strftime('%Y-%m', ts.start_ts, 'localtime')` for month buckets, `date(...)` for day buckets). Extend this same pattern for trend queries rather than introducing a new abstraction. HIGH confidence (verified directly against this repo's own `analytics.rs`, not web research). |

**Bucketing formulas to reuse (already established in `analytics.rs`):**
- **Day bucket:** `date(ts.start_ts, 'localtime')`
- **Month bucket:** `strftime('%Y-%m', ts.start_ts, 'localtime')`
- **Week bucket (Monday-aligned):** the codebase already computes "start of this week" with `date('now', 'localtime', '-' || ((CAST(strftime('%w','now','localtime') AS INTEGER) + 6) % 7) || ' days')`. Generalize this to an arbitrary column for a per-row weekly bucket:
  ```sql
  date(ts.start_ts, 'localtime',
       '-' || ((CAST(strftime('%w', ts.start_ts, 'localtime') AS INTEGER) + 6) % 7) || ' days'
  ) AS week_start
  ```
  This is the idiomatic SQLite substitute for `date_trunc('week', ...)` (Postgres) — SQLite's `%W` week-of-year code doesn't align to a clean Monday boundary the way this app's existing convention does, so stick with the offset-arithmetic pattern already in use rather than introducing `%W`.
- **Caveat (confirmed by research, MEDIUM confidence):** `strftime`-bucketed columns are TEXT, not a date type — they sort correctly only because ISO-8601 ordering happens to be lexicographic. This is fine for `GROUP BY`/`ORDER BY` but means no date arithmetic on the bucket column itself; keep deriving buckets from the raw timestamp column, not from a previously-bucketed string.
- **Streak-over-time:** keep following the existing pattern of pulling `SELECT DISTINCT date(...)` rows into Rust and computing streaks in Rust (as `current_streak()` already does, with existing unit tests) rather than trying to compute streak runs in SQL — SQLite has no native gaps-and-islands window function ergonomics that would make this simpler than the existing Rust approach.

### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| None required beyond uPlot | — | — | uPlot ships its own tooltip/cursor/legend plugins; no separate date library needed since all bucketing happens in SQL, not JS. Avoid adding `date-fns`/`dayjs`/`luxon` unless the frontend needs to format bucket labels beyond simple string slicing (`"2026-03"` → `"Mar 2026"` is a one-line `Intl.DateTimeFormat` call, not a library). |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Charting engine | uPlot | Chart.js | ~5x larger bundle (254KB vs 48KB) for a desktop app that needs only line/area/bar trend charts — Chart.js's broader chart-type catalog and easier declarative config aren't worth the size tax here since requirements are narrow (line trends, bar-style time-of-day, streak calendar). |
| Charting engine | uPlot | D3.js | D3 is a low-level toolkit (scales/axes/transitions from scratch), not a chart library — much higher implementation cost for the same three chart types, and its full bundle (~320KB gzip) is larger than uPlot even tree-shaken (~85KB core). Only worth it for fully bespoke visualizations this feature doesn't need. |
| Charting engine | uPlot | Recharts / other React-first libs | Wrong framework — this is a Svelte 5 app; React-wrapper libraries add an unnecessary React runtime dependency. |
| Charting engine | uPlot | Apache ECharts | ECharts is built for large/real-time/IoT-scale datasets with WebGL rendering; massive overkill (and bundle size) for a single-user local SQLite app with thousands, not millions, of rows. |
| Charting engine | uPlot | SciChart | The only library explicitly marketed with native Tauri support, but it's a commercial GPU-accelerated product aimed at financial/scientific real-time charting — wrong price point and wrong problem shape for this app. |
| Streak heatmap | LayerCake (optional) | Hand-rolled CSS grid | A plain CSS grid + `{#each}` over day buckets is a completely valid zero-dependency alternative for a GitHub-style streak calendar and should be tried first before reaching for LayerCake — only escalate to LayerCake if you need real scale/zoom/tooltip interaction on the heatmap. |
| Date bucketing | `strftime()`/`date()` in SQL (existing pattern) | JS-side bucketing (fetch raw rows, bucket in Svelte/TS) | SQLite aggregation pushes the bucketing + `SUM`/`COUNT` work to the database where indexes and `GROUP BY` do it efficiently, and matches the codebase's existing convention throughout `analytics.rs` — introducing JS-side bucketing would fork the pattern and duplicate the Monday-week-alignment logic in two languages. |

## Installation

```bash
# Core charting
npm install uplot

# Only if the streak heatmap needs real scale/interaction logic
# (try a hand-rolled CSS grid first)
npm install layercake

# No Rust/Cargo changes needed — rusqlite 0.40.2 already in Cargo.toml
# covers all required strftime()/date() bucketing.
```

No dev dependencies needed beyond what's already installed (TypeScript types for uPlot ship with the package).

## Sources

- uPlot GitHub repo and official benchmark data (github.com/leeoniya/uplot) — MEDIUM confidence, cross-checked across independent web sources
- LayerCake GitHub discussion #225 (mhkeller/layercake) on positioning vs Chart.js/ApexCharts — MEDIUM confidence
- SciChart, LightningChart, and general 2026 JS charting library comparison roundups (scichart.com, lightningchart.com, luzmo.com, embeddable.com) — MEDIUM confidence
- SQLite date/time bucketing idioms: LearnSQL.com, SQLHabit.com, Fivetran blog, SQLite official forum — MEDIUM confidence
- This repo's own `src-tauri/src/commands/analytics.rs` (existing Monday-week and month-bucket SQL patterns, `current_streak()` Rust implementation, `rusqlite` 0.40.2 in `Cargo.toml`) — HIGH confidence (direct code inspection, not web research)
- npm registry version checks for `uplot` (1.6.32), `layercake` (11.0.0), `chart.js` (4.5.1) as of 2026-09-20 — HIGH confidence (direct registry query)
