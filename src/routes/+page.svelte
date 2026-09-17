<script lang="ts">
  import TaskTreeNode from "$lib/components/TaskTreeNode.svelte";
  import PlannerPanel from "$lib/components/PlannerPanel.svelte";
  import { getToday, type TodaySummary } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatDurationShort, localDate } from "$lib/format";

  let today = $state<TodaySummary | null>(null);
  let error = $state("");
  let planning = $state(false);

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
    <div class="today-actions"><button onclick={() => planning = !planning}>{planning ? "Hide plan" : "Plan my day"}</button><span class="date">{todayDate}</span></div>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if planning}<PlannerPanel startDate={localDate()} days={1} onclose={() => planning = false} onapplied={refresh} />{/if}

  {#if today}
    <div class="summary">
      <span><strong>{today.task_count}</strong>Tasks to work on</span>
      <span><strong>{formatDurationShort(today.estimated_minutes_total * 60)}</strong>Estimated</span>
      <span><strong>{formatDurationShort(today.tracked_seconds_total)}</strong>Tracked today</span>
    </div>

    {#if today.next_up.length > 0}
      <section class="next-up">
        <h2>Next Up</h2>
        <ol>
          {#each today.next_up as task, i (task.id)}
            <li>
              <span class="rank">{i + 1}</span>
              <div>
                <strong>{task.title}</strong>
                <span class="meta">
                  {#if task.overdue}Overdue{:else if task.due_at}Due {new Date(task.due_at).toLocaleDateString(undefined, { month: "short", day: "numeric" })}{:else if task.scheduled_date === new Date().toISOString().slice(0, 10)}Scheduled today{/if}
                  {#if task.remaining_minutes !== null}· {task.remaining_minutes}m remaining{/if}
                </span>
              </div>
            </li>
          {/each}
        </ol>
      </section>
    {/if}

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
  .today-actions { display: flex; align-items: center; gap: 12px; }
  .date { color: var(--muted); font-size: 12px; }
  .summary { display: flex; gap: 28px; padding: 17px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); margin-bottom: 30px; }
  .summary span { display: flex; flex-direction: column; gap: 3px; color: var(--muted); font-size: 11px; }
  .summary strong { color: var(--text); font-size: 18px; letter-spacing: -.5px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .next-up { margin-bottom: 30px; }
  .next-up h2 { font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 0 0 10px; }
  .next-up ol { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 6px; }
  .next-up li { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border: 1px solid var(--line); border-radius: 8px; }
  .next-up .rank { font-size: 12px; font-weight: 700; color: var(--muted); width: 16px; }
  .next-up strong { display: block; font-size: 13px; }
  .next-up .meta { font-size: 11px; color: var(--muted); }
  .class-group { margin-bottom: 30px; }
  .class-group h2 { display: flex; align-items: center; gap: 10px; font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 0 0 8px; }
  .class-group h2::before { content: ""; width: 5px; height: 14px; border-radius: 2px; background: var(--accent); }
  ul { list-style: none; padding: 0; margin: 0; }
  .empty strong { color: var(--text); display: block; font-weight: 600; }
  .empty p { font-size: 12px; }
  @media(max-width: 540px) { header { align-items: start; flex-direction: column; gap: 3px; } .summary { gap: 20px; } }
</style>
