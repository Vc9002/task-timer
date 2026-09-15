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
  <div class="header">
    <button onclick={() => shiftDay(-1)}>&larr;</button>
    <h1>{new Date(date + "T00:00:00").toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" })}</h1>
    <button onclick={() => shiftDay(1)}>&rarr;</button>
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
            <input type="number" bind:value={editMinutes} class="edit-input" />
            <span>min</span>
            <button onclick={() => saveEdit(s.session_id)}>Save</button>
            <button onclick={() => (editingId = null)}>Cancel</button>
          {:else}
            <span class="duration">{formatDurationShort(s.duration_seconds)}</span>
            <button class="edit" onclick={() => startEdit(s.session_id, s.duration_seconds)}>Edit</button>
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
  .container {
    max-width: 720px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 0.5rem;
  }

  .header h1 {
    margin: 0;
    font-size: 1.3em;
    flex: 1;
    text-align: center;
  }

  .header button {
    background: #e5e5e5;
    color: #333;
    border: none;
    border-radius: 6px;
    padding: 0.3em 0.7em;
    cursor: pointer;
  }

  .total {
    font-weight: 600;
    margin-bottom: 1rem;
  }

  .by-class,
  .sessions {
    list-style: none;
    padding: 0;
    margin: 0 0 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .by-class li {
    display: flex;
    justify-content: space-between;
    padding: 0.4em 0.6em;
    background: white;
    border-radius: 6px;
    border: 1px solid #ececec;
  }

  .sessions li {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5em 0.6em;
    background: white;
    border-radius: 6px;
    border: 1px solid #ececec;
    font-size: 0.92em;
  }

  .time {
    color: #888;
    min-width: 130px;
  }

  .course {
    font-weight: 600;
  }

  .task {
    flex: 1;
  }

  .duration {
    font-variant-numeric: tabular-nums;
  }

  .edit,
  button {
    padding: 0.3em 0.7em;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #e5e5e5;
    color: #333;
    cursor: pointer;
  }

  .edit-input {
    width: 60px;
    padding: 0.2em 0.4em;
    border-radius: 4px;
    border: 1px solid #ccc;
  }

  .empty {
    color: #888;
    font-style: italic;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    .by-class li,
    .sessions li {
      background: #3a3a3a;
      border-color: #4a4a4a;
    }
  }
</style>
