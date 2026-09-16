<script lang="ts">
  import { onMount } from "svelte";
  import {
    getAnalyticsToday,
    getAnalyticsWeek,
    getAnalyticsMonth,
    type RangeSummary,
  } from "$lib/api";
  import { formatDurationShort } from "$lib/format";

  type RangeKey = "today" | "week" | "month";
  let range = $state<RangeKey>("week");
  let summary = $state<RangeSummary | null>(null);
  let error = $state("");

  async function load() {
    try {
      summary =
        range === "today"
          ? await getAnalyticsToday()
          : range === "week"
            ? await getAnalyticsWeek()
            : await getAnalyticsMonth();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  function setRange(r: RangeKey) {
    range = r;
    load();
  }

  onMount(load);
</script>

<main class="container">
  <h1>Analytics</h1>
  <p class="page-intro">See how your study time adds up.</p>

  <div class="tabs">
    <button class:active={range === "today"} onclick={() => setRange("today")}>Today</button>
    <button class:active={range === "week"} onclick={() => setRange("week")}>This Week</button>
    <button class:active={range === "month"} onclick={() => setRange("month")}>This Month</button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if summary}
    <p class="total">Total tracked<strong>{formatDurationShort(summary.tracked_seconds_total)}</strong></p>

    <ul class="by-class">
      {#each summary.by_class as c (c.class_id)}
        <li>
          <span>{c.course_code}</span>
          <span class="duration">{formatDurationShort(c.tracked_seconds)}</span>
          <div class="bar" aria-hidden="true"><span style:width={`${summary.tracked_seconds_total > 0 ? c.tracked_seconds / summary.tracked_seconds_total * 100 : 0}%`}></span></div>
        </li>
      {/each}
      {#if summary.by_class.length === 0}
        <li class="empty">No tracked time in this range yet.</li>
      {/if}
    </ul>
  {/if}
</main>

<style>
.tabs { display: inline-flex; gap: 3px; padding: 3px; border: 1px solid var(--line); border-radius: 8px; margin: 10px 0 25px; }
  .tabs button { border: 0; background: transparent; font-size: 12px; padding: 5px 12px; color: var(--muted); }
  .tabs button.active { background: var(--accent-soft); color: var(--accent); }
  .total { display: flex; flex-direction: column; gap: 6px; color: var(--muted); font-size: 12px; margin: 0 0 30px; }
  .total strong { color: var(--text); font-size: 34px; letter-spacing: -1.4px; font-weight: 600; }
  .by-class { list-style: none; padding: 0; }
  .by-class li { display: grid; grid-template-columns: 1fr auto; gap: 10px; padding: 18px 0; border-bottom: 1px solid var(--line); font-size: 13px; }
  .bar { grid-column: 1 / -1; height: 4px; background: var(--line); border-radius: 2px; overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--accent); border-radius: 2px; }
  .duration { font-variant-numeric: tabular-nums; color: var(--muted); }
</style>
