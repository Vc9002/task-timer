<script lang="ts">
  import { onMount } from "svelte";
  import { getDayView, editSessionDuration, type DayView } from "$lib/api";
  import { formatDurationShort } from "$lib/format";

  function todayIso() {
    return new Date().toLocaleDateString("en-CA"); // YYYY-MM-DD in local time
  }

  let date = $state(todayIso());
  let view = $state<DayView | null>(null);
  let error = $state("");
  let editingId = $state<number | null>(null);
  let editMinutes = $state("");

  async function load() {
    try {
      view = await getDayView(date);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  function shiftDay(delta: number) {
    const d = new Date(date + "T00:00:00");
    d.setDate(d.getDate() + delta);
    date = d.toLocaleDateString("en-CA");
    load();
  }

  function startEdit(sessionId: number, currentSeconds: number) {
    editingId = sessionId;
    editMinutes = (currentSeconds / 60).toFixed(0);
  }

  async function saveEdit(sessionId: number) {
    const minutes = Number(editMinutes);
    if (!Number.isFinite(minutes) || minutes < 0) return;
    try {
      await editSessionDuration(sessionId, Math.round(minutes * 60));
      editingId = null;
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function formatTime(iso: string) {
    return new Date(iso.replace(" ", "T") + "Z").toLocaleTimeString(undefined, {
      hour: "numeric",
      minute: "2-digit",
    });
  }

  onMount(load);
</script>

<main class="container">
  <h1>History</h1>
  <p class="page-intro">A record of where your time went.</p>
  <div class="header">
    <button aria-label="Previous day" onclick={() => shiftDay(-1)}>&larr;</button>
    <h2>{new Date(date + "T00:00:00").toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" })}</h2>
    <button aria-label="Next day" onclick={() => shiftDay(1)}>&rarr;</button>
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if view}
    <p class="total">Total: {formatDurationShort(view.total_seconds)}</p>

    <ul class="by-class">
      {#each view.by_class as c (c.class_id)}
        <li><span>{c.course_code}</span><span>{formatDurationShort(c.tracked_seconds)}</span></li>
      {/each}
    </ul>

    <h2>Sessions</h2>
    <ul class="sessions">
      {#each view.sessions as s (s.session_id)}
        <li>
          <span class="time">{formatTime(s.start_ts)}{s.end_ts ? ` – ${formatTime(s.end_ts)}` : ""}</span>
          <span class="course">{s.course_code}</span>
          <span class="task">{s.task_title}</span>
          {#if editingId === s.session_id}
            <input aria-label="Session duration in minutes" type="number" min="0" bind:value={editMinutes} class="edit-input" />
            <span>min</span>
            <button onclick={() => saveEdit(s.session_id)}>Save</button>
            <button onclick={() => (editingId = null)}>Cancel</button>
          {:else}
            <span class="duration">{formatDurationShort(s.duration_seconds)}</span>
            {#if s.end_ts}<button class="edit" aria-label={`Edit duration for ${s.task_title}`} onclick={() => startEdit(s.session_id, s.duration_seconds)}>Edit</button>{:else}<span class="time">In progress</span>{/if}
          {/if}
        </li>
      {/each}
      {#if view.sessions.length === 0}
        <li class="empty">No sessions tracked this day.</li>
      {/if}
    </ul>
  {/if}
</main>

<style>
.header { display: flex; align-items: center; gap: 12px; margin: 22px 0; }
  .header h2 { flex: 1; text-align: center; margin: 0; font-size: 14px; }
  .total { color: var(--muted); font-size: 13px; padding: 12px 0; border-bottom: 1px solid var(--line); }
  .by-class, .sessions { list-style: none; padding: 0; margin: 0 0 26px; }
  .by-class li { display: flex; justify-content: space-between; padding: 10px 0; border-bottom: 1px solid var(--line); font-size: 13px; }
  .sessions li { display: flex; align-items: center; flex-wrap: wrap; gap: 10px; padding: 14px 0; border-bottom: 1px solid var(--line); font-size: 12px; }
  .time { color: var(--muted); min-width: 110px; font-size: 11px; }
  .course { font-size: 10px; color: var(--accent); background: var(--accent-soft); padding: 2px 5px; border-radius: 4px; font-weight: 600; }
  .task { flex: 1; min-width: 110px; }
  .duration { font-variant-numeric: tabular-nums; font-weight: 600; }
  .edit { min-height: 25px; font-size: 11px; padding: 2px 7px; }
  .edit-input { width: 70px; }
</style>
