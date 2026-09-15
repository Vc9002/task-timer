<script lang="ts">
  import { onMount } from "svelte";
  import { getToday, type TodaySummary } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatDurationShort } from "$lib/format";

  let today = $state<TodaySummary | null>(null);
  let error = $state("");

  async function refresh() {
    try {
      today = await getToday(true);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function start(taskId: number) {
    const ok = await timerStore.start(taskId);
    if (ok) await refresh();
  }

  onMount(refresh);

  const todayDate = new Date().toLocaleDateString(undefined, {
    weekday: "long",
    month: "short",
    day: "numeric",
  });
</script>

<main class="container">
  <header>
    <h1>Today</h1>
    <span class="date">{todayDate}</span>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if today}
    <div class="summary">
      <span>{today.task_count} tasks</span>
      <span>Estimated: {formatDurationShort(today.estimated_minutes_total * 60)}</span>
      <span>Tracked: {formatDurationShort(today.tracked_seconds_total)}</span>
    </div>

    {#if timerStore.conflict}
      <div class="conflict">
        You're currently tracking {timerStore.conflict.classCode} — {timerStore.conflict.taskTitle}.
        <button onclick={async () => { await timerStore.finish(); timerStore.conflict = null; await refresh(); }}>
          Stop & continue
        </button>
        <button onclick={() => (timerStore.conflict = null)}>Cancel</button>
      </div>
    {/if}

    {#each today.groups as group (group.class_id)}
      <section class="class-group">
        <h2>{group.course_code}</h2>
        <ul>
          {#each group.tasks.filter((t) => !t.parent_task_id) as task (task.id)}
            <li class="task-row">
              <span class="status-dot" class:done={task.status === "completed"}></span>
              <span class="title">
                {task.title}
                {#if task.source === "todoist"}<span class="badge">Todoist</span>{/if}
              </span>
              {#if task.estimated_minutes}
                <span class="estimate">{task.estimated_minutes}m estimated</span>
              {/if}
              {#if task.tracked_seconds > 0}
                <span class="tracked">{formatDurationShort(task.tracked_seconds)}</span>
              {/if}
              {#if !timerStore.active && task.status !== "completed"}
                <button onclick={() => start(task.id)}>Start</button>
              {/if}
            </li>
            {#each group.tasks.filter((t) => t.parent_task_id === task.id) as sub (sub.id)}
              <li class="task-row subtask">
                <span class="status-dot" class:done={sub.status === "completed"}></span>
                <span class="title">{sub.title}</span>
                {#if sub.tracked_seconds > 0}
                  <span class="tracked">{formatDurationShort(sub.tracked_seconds)}</span>
                {/if}
                {#if !timerStore.active && sub.status !== "completed"}
                  <button onclick={() => start(sub.id)}>Start</button>
                {/if}
              </li>
            {/each}
          {/each}
        </ul>
      </section>
    {/each}

    {#if today.groups.length === 0}
      <p class="empty">Nothing on Today. Add a task in Classes, or check Todoist sync in Settings.</p>
    {/if}
  {/if}
</main>

<style>
  .container {
    max-width: 720px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem;
  }

  header {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }

  h1 {
    margin: 0;
  }

  .date {
    color: #888;
  }

  .summary {
    display: flex;
    gap: 1.25rem;
    color: #666;
    margin-bottom: 1.5rem;
    font-size: 0.95em;
  }

  .conflict {
    background: #fff4e0;
    border: 1px solid #e0b060;
    border-radius: 8px;
    padding: 0.75rem 1rem;
    margin-bottom: 1.5rem;
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .class-group {
    margin-bottom: 1.75rem;
  }

  .class-group h2 {
    font-size: 1em;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #666;
    border-bottom: 1px solid #e5e5e5;
    padding-bottom: 0.4rem;
    margin-bottom: 0.5rem;
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.6rem;
    background: white;
    border-radius: 6px;
    border: 1px solid #ececec;
  }

  .task-row.subtask {
    margin-left: 1.5rem;
    background: #fafafa;
  }

  .status-dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    border: 2px solid #396cd8;
    flex-shrink: 0;
  }

  .status-dot.done {
    background: #396cd8;
  }

  .title {
    flex: 1;
  }

  .badge {
    font-size: 0.7em;
    color: #888;
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 0.05em 0.4em;
    margin-left: 0.4em;
  }

  .estimate,
  .tracked {
    font-size: 0.85em;
    color: #888;
  }

  button {
    padding: 0.35em 0.8em;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #396cd8;
    color: white;
    cursor: pointer;
  }

  .empty {
    color: #888;
    font-style: italic;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    .task-row {
      background: #3a3a3a;
      border-color: #4a4a4a;
    }
    .task-row.subtask {
      background: #333;
    }
    .class-group h2 {
      border-color: #4a4a4a;
    }
  }
</style>
