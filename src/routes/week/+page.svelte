<script lang="ts">
  import { getWeek, scheduleTask, type WeekSummary, type WeekTask } from "$lib/api";
  import PlannerPanel from "$lib/components/PlannerPanel.svelte";
  import { formatMinutesShort, formatDurationShort, localDate } from "$lib/format";
  import { timerStore } from "$lib/stores/timer.svelte";

  let week = $state<WeekSummary | null>(null);
  let error = $state("");
  let startDate = $state(mondayOf(new Date()));
  let reschedulingId = $state<number | null>(null);
  let planning = $state(false);

  function mondayOf(date: Date): string {
    const d = new Date(date);
    const day = (d.getDay() + 6) % 7; // 0 = Monday
    d.setDate(d.getDate() - day);
    return localDate(d);
  }

  async function refresh() {
    try {
      week = await getWeek(startDate);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => { void timerStore.revision; void startDate; void refresh(); });

  function shiftWeek(deltaDays: number) {
    const d = new Date(startDate);
    d.setDate(d.getDate() + deltaDays);
    startDate = localDate(d);
  }

  function dayLabel(dateStr: string): { weekday: string; day: string } {
    const d = new Date(dateStr + "T00:00:00");
    return {
      weekday: d.toLocaleDateString(undefined, { weekday: "long" }),
      day: d.toLocaleDateString(undefined, { month: "short", day: "numeric" }),
    };
  }

  function loadClass(percent: number | null): string {
    if (percent === null) return "";
    if (percent > 100) return "overloaded";
    if (percent >= 85) return "heavy";
    return "normal";
  }

  async function reschedule(task: WeekTask, date: string | null) {
    reschedulingId = null;
    await scheduleTask(task.id, date);
    await refresh();
  }

  function offsetDate(base: string, days: number): string {
    const d = new Date(base + "T00:00:00");
    d.setDate(d.getDate() + days);
    return localDate(d);
  }

  const today = localDate();
</script>

<main class="container">
  <header>
    <div>
      <p class="eyebrow">Academic planning</p>
      <h1>Week</h1>
    </div>
    <div class="nav">
      <button class="plan-button" onclick={() => planning = !planning}>{planning ? "Hide plan" : "Plan my week"}</button>
      <button onclick={() => shiftWeek(-7)} aria-label="Previous week">←</button>
      <button onclick={() => (startDate = mondayOf(new Date()))}>This week</button>
      <button onclick={() => shiftWeek(7)} aria-label="Next week">→</button>
    </div>
  </header>

  {#if error}<p class="error">{error}</p>{/if}

  {#if planning}<PlannerPanel startDate={startDate} days={7} onclose={() => planning = false} onapplied={refresh} />{/if}

  {#if week}
    <div class="summary">
      <span><strong>{week.days.reduce((n, d) => n + d.tasks.length, 0)}</strong>Tasks scheduled</span>
      <span><strong>{formatMinutesShort(week.estimated_minutes_remaining)}</strong>Estimated remaining</span>
      <span><strong>{formatDurationShort(week.tracked_seconds)}</strong>Tracked</span>
    </div>

    <div class="heatmap">
      {#each week.days as day (day.date)}
        {@const label = dayLabel(day.date)}
        <div class="bar-row">
          <span class="bar-label">{label.weekday.slice(0, 3)}</span>
          <div class="bar-track">
            {#if day.load_percent !== null}
              <div class="bar-fill {loadClass(day.load_percent)}" style="width: {Math.min(100, day.load_percent)}%"></div>
            {/if}
          </div>
          <span class="bar-pct">{day.load_percent !== null ? `${day.load_percent}%` : "—"}</span>
        </div>
      {/each}
    </div>

    {#each week.days as day (day.date)}
      {@const label = dayLabel(day.date)}
      {@const overloaded = day.load_percent !== null && day.load_percent > 100}
      <section class="day">
        <h2>
          {label.weekday.toUpperCase()} {label.day}
          {#if day.date === today}<span class="today-badge">Today</span>{/if}
          {#if day.tasks.length > 0 || day.study_blocks.length > 0}<span class="day-meta">{formatMinutesShort(day.estimated_minutes_remaining)} remaining · {day.study_block_minutes}m blocked</span>{/if}
        </h2>

        {#if overloaded}
          <p class="warning">
            {label.weekday} is overloaded — {formatMinutesShort(day.estimated_minutes_remaining)} planned against {formatMinutesShort(day.capacity_minutes ?? 0)} available.
          </p>
        {/if}

        {#if day.study_blocks.length > 0}
          <div class="blocks"><p class="section-label">STUDY BLOCKS</p>{#each day.study_blocks as block (block.id)}<div class="block"><span class="course">{block.course_code}</span><strong>{block.task_title}</strong><span class="meta">{block.planned_start_time ? `${block.planned_start_time} · ` : ""}{block.planned_minutes}m</span></div>{/each}</div>
        {/if}
        {#if day.tasks.length === 0 && day.study_blocks.length === 0}
          <p class="empty-day">Nothing scheduled.</p>
        {:else if day.tasks.length > 0}
          <ul>
            {#each day.tasks as task (task.id)}
              <li class:completed={task.status === "completed"}>
                <div class="task-main">
                  <span class="course">{task.course_code}</span>
                  <strong>{#if task.parent_path}<span class="breadcrumb">{task.parent_path} › </span>{/if}{task.title}</strong>
                  <span class="meta">
                    {#if task.remaining_minutes !== null}{task.remaining_minutes}m remaining{/if}
                    {#if task.due_at}· Due {new Date(task.due_at).toLocaleDateString(undefined, { weekday: "short" })}{/if}
                    {#if task.unplanned_minutes > 0}· ⚠ {task.unplanned_minutes}m gap{/if}
                  </span>
                </div>
                <div class="reschedule">
                  <button onclick={() => (reschedulingId = reschedulingId === task.id ? null : task.id)}>Reschedule</button>
                  {#if reschedulingId === task.id}
                    <div class="popover">
                      <button onclick={() => reschedule(task, today)}>Today</button>
                      <button onclick={() => reschedule(task, offsetDate(today, 1))}>Tomorrow</button>
                      <button onclick={() => reschedule(task, null)}>Unschedule</button>
                    </div>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/each}

    {#if week.unscheduled.length > 0}
      <section class="unscheduled">
        <h2>UNSCHEDULED</h2>
        <ul>
          {#each week.unscheduled as task (task.id)}
            <li>
              <div class="task-main">
                <span class="course">{task.course_code}</span>
                <strong>{task.title}</strong>
                <span class="meta">
                  {#if task.due_at}Due {new Date(task.due_at).toLocaleDateString(undefined, { weekday: "short" })}{/if}
                  {#if task.estimated_minutes}· {task.estimated_minutes}m estimate{/if}
                </span>
              </div>
              <div class="quick-schedule">
                <button onclick={() => reschedule(task, today)}>Today</button>
                {#each week.days.slice(1, 3) as day (day.date)}
                  <button onclick={() => reschedule(task, day.date)}>{dayLabel(day.date).weekday.slice(0, 3)}</button>
                {/each}
                <button onclick={() => (reschedulingId = reschedulingId === task.id ? null : task.id)}>Pick date</button>
              </div>
              {#if reschedulingId === task.id}
                <input
                  type="date"
                  onchange={(e) => reschedule(task, (e.target as HTMLInputElement).value)}
                />
              {/if}
            </li>
          {/each}
        </ul>
      </section>
    {/if}
  {:else if !error}<p class="page-intro" role="status">Loading week…</p>{/if}
</main>

<style>
  header { display: flex; justify-content: space-between; align-items: end; gap: 15px; margin-bottom: 24px; }
  .nav { display: flex; gap: 6px; }
  .nav button { border: 1px solid var(--line); background: var(--surface); border-radius: 6px; padding: 6px 10px; font-size: 12px; cursor: pointer; }
  .summary { display: flex; gap: 28px; padding: 17px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); margin-bottom: 20px; }
  .summary span { display: flex; flex-direction: column; gap: 3px; color: var(--muted); font-size: 11px; }
  .section-label { margin: 0 0 5px; color: var(--muted); font-size: 9px; letter-spacing: .08em; font-weight: 700; }
  .blocks { margin-bottom: 12px; padding-bottom: 8px; border-bottom: 1px solid var(--line); }
  .block { display: flex; gap: 8px; align-items: baseline; padding: 5px 0; font-size: 12px; }
  .block strong { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .summary strong { color: var(--text); font-size: 18px; letter-spacing: -.5px; font-weight: 600; font-variant-numeric: tabular-nums; }

  .heatmap { display: flex; flex-direction: column; gap: 4px; margin-bottom: 26px; }
  .bar-row { display: grid; grid-template-columns: 32px 1fr 44px; align-items: center; gap: 8px; font-size: 11px; color: var(--muted); }
  .bar-track { height: 8px; border-radius: 4px; background: var(--hover); overflow: hidden; }
  .bar-fill { height: 100%; border-radius: 4px; }
  .bar-fill.normal { background: var(--accent); }
  .bar-fill.heavy { background: #d9a441; }
  .bar-fill.overloaded { background: #d9534f; }
  .bar-pct { text-align: right; font-variant-numeric: tabular-nums; }

  .day { margin-bottom: 22px; }
  .day h2 { display: flex; align-items: center; gap: 10px; font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 0 0 8px; }
  .today-badge { color: var(--accent); font-weight: 700; }
  .day-meta { margin-left: auto; text-transform: none; letter-spacing: normal; }
  .warning { font-size: 12px; color: #d9534f; margin: 0 0 8px; }
  .empty-day { font-size: 12px; color: var(--muted); margin: 0; }

  ul { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
  li { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 10px 12px; border: 1px solid var(--line); border-radius: 8px; position: relative; flex-wrap: wrap; }
  li.completed { opacity: .5; }
  .task-main { display: flex; flex-direction: column; gap: 2px; }
  .course { font-size: 10px; text-transform: uppercase; color: var(--muted); }
  .meta { font-size: 11px; color: var(--muted); }

  .reschedule { position: relative; }
  .reschedule > button, .quick-schedule button { border: 1px solid var(--line); background: var(--surface); border-radius: 6px; padding: 5px 9px; font-size: 11px; cursor: pointer; }
  .popover { position: absolute; right: 0; top: 100%; margin-top: 4px; display: flex; flex-direction: column; gap: 2px; background: var(--surface); border: 1px solid var(--line); border-radius: 8px; padding: 4px; z-index: 5; }
  .popover button { text-align: left; border: none; background: none; padding: 6px 8px; font-size: 12px; cursor: pointer; border-radius: 4px; }
  .popover button:hover { background: var(--hover); }
  .quick-schedule { display: flex; gap: 4px; flex-wrap: wrap; }

  .unscheduled h2 { font-size: 11px; text-transform: uppercase; letter-spacing: .08em; color: var(--muted); margin: 0 0 8px; }
  @media(max-width: 540px) { header { align-items: start; flex-direction: column; gap: 8px; } .summary { gap: 20px; } }
</style>
