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

  <div class="tabs">
    <button class:active={range === "today"} onclick={() => setRange("today")}>Today</button>
    <button class:active={range === "week"} onclick={() => setRange("week")}>This Week</button>
    <button class:active={range === "month"} onclick={() => setRange("month")}>This Month</button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if summary}
    <p class="total">Total tracked: {formatDurationShort(summary.tracked_seconds_total)}</p>

    <ul class="by-class">
      {#each summary.by_class as c (c.class_id)}
        <li>
          <span>{c.course_code}</span>
          <span>{formatDurationShort(c.tracked_seconds)}</span>
        </li>
      {/each}
      {#if summary.by_class.length === 0}
        <li class="empty">No tracked time in this range yet.</li>
      {/if}
    </ul>
  {/if}
</main>

<style>
  .container {
    max-width: 640px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem;
  }

  .tabs {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1.25rem;
  }

  .tabs button {
    padding: 0.4em 0.9em;
    border-radius: 6px;
    border: 1px solid #ccc;
    background: white;
    cursor: pointer;
  }

  .tabs button.active {
    background: #396cd8;
    color: white;
    border-color: #396cd8;
  }

  .total {
    font-weight: 600;
    margin-bottom: 1rem;
  }

  .by-class {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .by-class li {
    display: flex;
    justify-content: space-between;
    padding: 0.6em 0.9em;
    background: white;
    border-radius: 6px;
    border: 1px solid #ececec;
  }

  .empty {
    color: #888;
    font-style: italic;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    .tabs button {
      background: #3a3a3a;
      border-color: #555;
      color: #f6f6f6;
    }
    .by-class li {
      background: #3a3a3a;
      border-color: #4a4a4a;
    }
  }
</style>
