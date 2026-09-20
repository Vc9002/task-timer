# Feature Landscape

**Domain:** Longitudinal trends/insights for a single-user local time-tracking desktop app (per-class study-time trends, streak history, time-of-day patterns)
**Researched:** 2026-09-20
**Confidence:** MEDIUM (cross-checked web search across 5 independent mature products: Toggl Track, RescueTime, WakaTime, Forest, Obsidian habit/heatmap plugin ecosystem)

## Table Stakes

Features users of mature time-tracking / study-analytics apps expect. Missing = the "trends" feature feels incomplete or like a snapshot tool pretending to be an insights tool.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Week-over-week / period-over-period comparison | Every mature tool (Toggl's Totals widget "trends and averages", RescueTime's daily/weekly/monthly views, WakaTime's "historical charts that compare current habits against previous periods") treats this as baseline. Users want "am I doing more or less than last week" without doing mental math. | Low–Med | TaskTimer already computes weekly/monthly snapshots — this is mostly a query that diffs two snapshot periods and a UI delta indicator (up/down %). |
| Per-category (per-class) time breakdown trended over time | Toggl's pivot-table-by-project/client, WakaTime's per-language/per-project trend, RescueTime's per-category trends are all standard. For TaskTimer, "per-class" is the direct analog. | Low–Med | Backend already has priority/class breakdowns per period; trending = same breakdown repeated across N periods, charted as stacked/grouped bars or small multiples. |
| Streak calendar / contribution-graph heatmap (GitHub-style) | This is the single most consistent pattern across the whole habit-tracking ecosystem — RescueTime's mobile focus-depth calendar heatmap, nearly every Obsidian habit plugin (Heatmap Tracker, Habit Tracker Pro, Activity Heatmap, Habit Heatmap Calendar), and general habit-tracker apps all converge on the same visual: a day-cell grid colored by intensity, scrollable across weeks/months/year. | Med | Requires a calendar-heatmap component (day-of-week rows × week columns, color scale by minutes tracked or streak status) — this is the highest-value differentiator-adjacent table-stake since it's now the expected way to show "history of a streak," not just a numeric current-streak count. |
| Streak history (not just current count) | User's own requirement already names this. Confirmed as standard: RescueTime's Focus Streak + calendar heatmap, Forest's "streaks... by day, week, or month," Duolingo-style streak calendars used broadly in habit apps. Showing only "current streak: 5 days" without a visual history of past streaks (including broken ones) is now considered incomplete. | Low–Med | Pair the heatmap (above) with a simple "streak timeline" — list/sparkline of past streak runs and their lengths. |
| Time-of-day / hour-of-day heatmap | RescueTime's hourly productivity chart ("breakdown of how productive you are each hour"), WakaTime's day/hour pattern tracking (explicitly surfaces "I thought I was a morning person but I'm not"), and the API-level `weekdays`/`days`/`best_day` insight types all point to this as expected, not exotic. | Med | Typically a 7×24 grid (day-of-week × hour) shaded by cumulative minutes tracked in that slot, aggregated across all history or a rolling window. This directly satisfies the user's third active requirement. |
| Trend charts with selectable time range (week/month/quarter/custom) | Universal across Toggl (custom-range Workload reports), RescueTime (daily/weekly/monthly/custom), WakaTime (`last_7_days`, `last_30_days`, `last_6_months`, `last_year`, `all_time`). A trends view locked to one fixed window (e.g., only "last 8 weeks") feels rigid compared to competitors. | Low–Med | Reuse existing snapshot period logic; add a range selector in the UI layer, not new backend logic per range. |
| "Most productive day / best day" surfacing | RescueTime's weekly email explicitly calls out "most productive day and time... which day you did your best work." WakaTime's `best_day` insight type is a first-class API field. This is a cheap, high-perceived-value derived stat once trend data exists. | Low | Just a MAX() over the per-day aggregation already needed for the heatmap — near-zero extra backend cost once time-of-day data exists. |

## Differentiators

Features that set a trends feature apart. Valued, but many mature apps skip or paywall them — worth considering for TaskTimer given it's a student-focused, single-user, local app where you control 100% of the roadmap.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Cross-class time-of-day overlay ("which class do I actually study at night vs. morning") | Goes beyond generic time-of-day pattern to combine it with the class dimension — no competitor reviewed does this well since none of them have a "class" concept; this is TaskTimer-specific differentiation on top of a Toggl/RescueTime-style base pattern. | Med | Filter/facet the hour-of-day heatmap by class; reuses both existing dimensions, no new data collection needed. |
| Streak "personal best" and streak-break annotations | Habit-tracker apps show current + sometimes longest streak, but rarely annotate *why* a streak broke (e.g., exam week, no sessions logged) or show a "personal best" banner. Cheap motivational payoff for a single student user. | Low | Longest-streak = MAX over streak-run lengths, already computable from the same data as the heatmap. |
| Estimate-vs-actual trend (planned vs. tracked time per class over weeks) | TaskTimer's PROJECT.md notes it already tracks estimates; trending estimate accuracy over time (not just per-task) is a differentiator no reviewed competitor offers, since it's specific to task-estimation apps rather than pure time trackers. | Med | Requires joining existing estimate data with tracked-time trend data across periods — natural extension of existing backend fields. |
| Pomodoro cycle-completion trend (work/break adherence over time) | TaskTimer has Pomodoro mode; trending "how often are Pomodoro cycles completed vs. abandoned" over weeks is unique to Pomodoro-capable tools and not something Toggl/RescueTime (which aren't Pomodoro-native) surface. | Med | Depends on Pomodoro session data already being logged; needs a "completed vs. interrupted" trend view. |
| Annotated trend line (deadline/exam markers overlaid on time-trend chart) | Not offered by any competitor reviewed (none track academic deadlines). Would let a student see "time spent on Class X spiked right before the midterm" on the same chart. | Med–High | Needs exam/deadline dates plotted as chart annotations on top of the trend chart — moderate complexity, high student-specific value. Note: correlating with grades is explicitly out of scope per PROJECT.md, but showing deadline proximity without grade data is in-bounds and safe. |
| Shareable/exportable weekly digest (like RescueTime's and WakaTime's auto-email summary) | High-value pattern in mature tools, but explicitly a "social/sharing" pattern (email digest) — PROJECT.md says no social/sharing features needed. | — | **Recommend deferring** — matches the anti-feature category below since it's cloud/email delivery, not core to a local single-user app. Could be revisited later as a local "weekly summary" screen (not emailed) if desired — that variant would be a differentiator worth a small footnote, not a full email pipeline. |

## Anti-Features

Features to explicitly NOT build for this milestone.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|---------------------|
| Social/team comparison, leaderboards, shared dashboards | PROJECT.md explicitly scopes this as a single-user local app with "no social/sharing features needed." Toggl and RescueTime's team features (workspace-wide reports, shared dashboards) are irrelevant here. | Keep all trend views scoped to the single local user/database. |
| Cloud-synced email digests / push notifications for trends | Requires a backend/cloud delivery mechanism TaskTimer doesn't have and isn't planned to add ("Local-only" constraint in PROJECT.md: no new cloud/remote analytics services). RescueTime and WakaTime's weekly emails are the flagship example of this pattern. | If a "weekly summary" is wanted, render it as an in-app screen/card the user opens, not a scheduled email. |
| Grade/outcome correlation on trend charts | Explicitly out of scope per PROJECT.md ("Grade/outcome correlation... explicitly deferred"). | Trend views should show time/streak/pattern data only — no grade or outcome fields, even as an overlay. |
| Workload forecasting (predicting future time needs from trend data) | Explicitly out of scope per PROJECT.md ("Workload forecasting... explicitly deferred"). | Trends should be retrospective/descriptive only (what happened), not predictive (what will happen). Save forecasting for a future milestone. |
| Gamification layer (trees, XP, badges, streak "rewards") beyond simple streak counts | Forest's tree-growing metaphor and badge systems (seen in Obsidian's Habit Tracker Pro "badges") are popular but add scope (art assets, reward logic) disproportionate to a study-analytics feature request that's about *visibility* into existing data, not motivation mechanics. | Keep streak visualization data-focused (heatmap + streak-length stats), skip decorative/reward layers. |
| New third-party integrations to enrich trend data (Canvas, Google Calendar, etc.) | Explicitly out of scope per PROJECT.md ("New integrations beyond Todoist... explicitly deferred"). | Trends must be computable entirely from existing local SQLite data (tasks, sessions, classes) already captured by the current analytics backend. |
| Real-time/live-updating trend dashboards (sub-minute refresh) | None of the mature tools reviewed treat trends as real-time — Toggl, RescueTime, and WakaTime all trend on daily/weekly aggregates computed periodically, not live streams. Building real-time aggregation would be substantial added complexity with no evidenced user demand. | Compute/refresh trend data on app open or on a periodic interval consistent with the existing daily/weekly/monthly snapshot cadence already in `analytics.rs`. |

## Feature Dependencies

```
Existing daily/weekly/monthly snapshots (analytics.rs)
  → Week-over-week / period comparison (diff two snapshot periods)
  → Per-class trend charts (repeat existing class breakdown across N periods)
  → "Most productive day" surfacing (derived from time-of-day heatmap data)

Time-of-day / hour-of-day data collection (NEW: needs session start/end times bucketed by hour)
  → Time-of-day heatmap (7x24 grid)
  → Cross-class time-of-day overlay (differentiator, filters heatmap by class)

Streak data (existing streak counts in analytics.rs)
  → Streak calendar heatmap (NEW: needs per-day tracked/not-tracked history, not just current count)
  → Streak history / personal-best annotations (derived from heatmap day-cell data)

Existing estimate data (per task)
  → Estimate-vs-actual trend (differentiator, joins estimates with trended tracked time)

Existing Pomodoro session logging
  → Pomodoro cycle-completion trend (differentiator)

Existing exam countdown data
  → Deadline-annotated trend chart (differentiator, plots exam dates as chart markers)
```

**Key implication for backend work:** the current `analytics.rs` snapshot model likely does NOT retain enough day-level and hour-level granularity to power the streak heatmap and time-of-day heatmap directly from existing snapshot tables — those two table-stakes features probably need a query against the underlying session/task-log rows (not the precomputed snapshots), bucketed by day and by hour respectively. Confirm this during phase planning/architecture research, since it affects whether this is "new queries only" or "new queries + a new day-level aggregation table" scope.

## MVP Recommendation

Prioritize (all three match the user's stated Active requirements and are consistently table-stakes across the ecosystem):

1. **Per-class study-time trend charts across weeks/months** (week-over-week comparison + per-class breakdown trended) — directly satisfies stated requirement #1, lowest complexity since it's a direct extension of existing snapshot data.
2. **Streak calendar heatmap (GitHub-style)** — directly satisfies stated requirement #2 ("streak visualization over time, not just current count"); this is the most universally-expected pattern found across every category of app researched (time trackers, focus apps, habit trackers, note-taking plugins).
3. **Time-of-day / hour-of-day heatmap** — directly satisfies stated requirement #3; standard in RescueTime and WakaTime, and cheaply yields the bonus "most productive day/time" stat.

Defer (differentiators, not required for this milestone but worth flagging for roadmap discussion):
- **Cross-class time-of-day overlay**: natural follow-on once the base time-of-day heatmap exists — low incremental cost, consider bundling if time allows.
- **Estimate-vs-actual trend**, **Pomodoro cycle-completion trend**, **deadline-annotated trend chart**: genuine differentiators unique to TaskTimer's domain (task estimation + Pomodoro + exam tracking), but each adds its own data-joining complexity — reasonable candidates for a later phase or milestone, not required to make the trends feature feel complete.
- **In-app (non-email) weekly summary card**: nice-to-have echo of RescueTime/WakaTime's email digest pattern, minus the cloud delivery — low priority relative to the three core views.

## Sources

- [Toggl Track features](https://toggl.com/track/features/) — MEDIUM confidence (cross-checked)
- [Introducing Analytics: Time Tracking Reports For Better Insights (Toggl)](https://toggl.com/blog/introducing-analytics) — MEDIUM confidence
- [New Toggl Track Reporting Experience](https://toggl.com/blog/new-reporting-experience) — MEDIUM confidence
- [Toggl Track Reporting & Insights](https://toggl.com/track/reporting/) — MEDIUM confidence
- [RescueTime Dashboard help article](https://help.rescuetime.com/article/30-dashboard) — MEDIUM confidence (cross-checked)
- [Understanding Your Data (RescueTime)](https://help.rescuetime.com/article/461-understanding-your-data) — MEDIUM confidence
- [The Productivity Report (RescueTime)](https://help.rescuetime.com/article/61-the-productivity-report) — MEDIUM confidence
- [How to Use RescueTime to Get Magnitudes More Done Each Week (Zapier)](https://zapier.com/blog/rescuetime/) — MEDIUM confidence
- [WakaTime API Docs](https://wakatime.com/developers) — MEDIUM confidence (cross-checked)
- [WakaTime — Dashboards for developers](https://wakatime.com/wrapped) — MEDIUM confidence
- [WakaTime 2024 Programming Stats](https://wakatime.com/blog/68-wakatime-2024-programming-stats) — MEDIUM confidence
- [Forest App official site](https://forestapp.cc/) — MEDIUM confidence (cross-checked)
- [Forest vs Flora comparison (Nerdynav)](https://nerdynav.com/forest-vs-flora-pomodoro/) — LOW-MEDIUM confidence (third-party review, cross-checked against official site claims)
- [Heatmap Tracker — Obsidian Plugin](https://github.com/mokkiebear/heatmap-tracker) — MEDIUM confidence
- [Habits — Obsidian Plugin (obsidianstats)](https://www.obsidianstats.com/plugins/heatmap-tracker) — MEDIUM confidence
- [Habit Tracker Pro for Obsidian](https://github.com/msdanyg/habit-tracker-obsidian) — MEDIUM confidence
- Confidence tiers assigned via `gsd_run query classify-confidence --provider websearch [--verified]`: unverified web search = LOW; cross-checked across 3+ independent sources = MEDIUM (applied here since every claim above was corroborated across at least 2 of the 5 products researched).
