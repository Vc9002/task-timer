<script lang="ts">
  import TaskTreeNode from "$lib/components/TaskTreeNode.svelte";
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

  $effect(() => { void timerStore.revision; void refresh(); });

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

    {#each today.groups as group (group.class_id)}
      <section class="class-group">
        <h2>{group.course_code}</h2>
        <ul>
          {#each group.tasks.filter(t => !group.tasks.some(p => p.id === t.parent_task_id)) as task (task.id)}
            <TaskTreeNode {task} tasks={group.tasks} courseCode={group.course_code} onchanged={refresh} />
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

  .empty {
    color: #888;
    font-style: italic;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) { .class-group h2 { border-color: #4a4a4a; } }
</style>
