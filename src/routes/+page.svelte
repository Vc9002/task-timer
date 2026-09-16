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
    <div><p class="eyebrow">Your coursework</p><h1>Today</h1></div>
    <span class="date">{todayDate}</span>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if today}
    <div class="summary">
      <span><strong>{today.task_count}</strong>Tasks to work on</span>
      <span><strong>{formatDurationShort(today.estimated_minutes_total * 60)}</strong>Estimated</span>
      <span><strong>{formatDurationShort(today.tracked_seconds_total)}</strong>Tracked today</span>
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
      <div class="empty"><strong>A clear day ahead.</strong><p>Add a task with Cmd/Ctrl+K, or schedule existing coursework for today.</p><a href="/classes">Browse your classes →</a></div>
    {/if}
  {:else if !error}<p class="page-intro" role="status">Loading today's work…</p>{/if}
</main>

<style>
header { display: flex; justify-content: space-between; align-items: end; gap: 15px; margin-bottom: 24px; }
  .date { color: var(--muted); font-size: 12px; }
  .summary { display: flex; gap: 28px; padding: 17px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); margin-bottom: 30px; }
  .summary span { display: flex; flex-direction: column; gap: 3px; color: var(--muted); font-size: 11px; }
  .summary strong { color: var(--text); font-size: 18px; letter-spacing: -.5px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .class-group { margin-bottom: 30px; }
  .class-group h2 { display: flex; align-items: center; gap: 10px; font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 0 0 8px; }
  .class-group h2::before { content: ""; width: 5px; height: 14px; border-radius: 2px; background: var(--accent); }
  ul { list-style: none; padding: 0; margin: 0; }
  .empty strong { color: var(--text); display: block; font-weight: 600; }
  .empty p { font-size: 12px; }
  @media(max-width: 540px) { header { align-items: start; flex-direction: column; gap: 3px; } .summary { gap: 20px; } }
</style>
