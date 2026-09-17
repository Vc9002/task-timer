<script lang="ts">
  import {
    createStudyBlock, createTaskMilestone, deleteStudyBlock, deleteTaskMilestone,
    listStudyBlocksForTask, listTaskMilestones, moveTaskMilestone, updateStudyBlock,
    updateTaskMilestone, type StudyBlock, type TaskMilestone, type TaskRecord,
  } from "$lib/api";
  import { formatMinutesShort } from "$lib/format";
  let { task, onchanged }: { task: TaskRecord; onchanged: () => Promise<void> } = $props();
  let blocks = $state<StudyBlock[]>([]);
  let milestones = $state<TaskMilestone[]>([]);
  let title = $state("");
  let date = $state("");
  let startTime = $state("");
  let minutes = $state(30);
  let milestoneTitle = $state("");
  let milestoneDate = $state("");
  let error = $state("");
  let busy = $state(false);
  function timeMinutes(value: string | null) { if (!value) return null; const [hours, mins] = value.split(":").map(Number); return hours * 60 + mins; }
  function overlaps(dateValue: string, startValue: string | null, duration: number, excludeId?: number) {
    const start = timeMinutes(startValue); if (start === null) return false;
    return blocks.some(block => block.id !== excludeId && block.planned_date === dateValue && timeMinutes(block.planned_start_time) !== null && start < (timeMinutes(block.planned_start_time)! + block.planned_minutes) && (start + duration) > timeMinutes(block.planned_start_time)!);
  }

  async function refresh() {
    try {
      if (!date) date = task.due_at?.slice(0, 10) ?? "";
      [blocks, milestones] = await Promise.all([listStudyBlocksForTask(task.id), listTaskMilestones(task.id)]);
      error = "";
    } catch { error = "Couldn't load planning details."; }
  }
  $effect(() => { void task.id; void refresh(); });
  async function run(action: () => Promise<unknown>) {
    if (busy) return;
    busy = true; error = "";
    try { await action(); await refresh(); await onchanged(); }
    catch { error = "Couldn't save this planning change."; }
    finally { busy = false; }
  }
  function blockLabel(block: StudyBlock) { return block.planned_start_time ? `${block.planned_start_time} · ` : ""; }
</script>

<div class="planning">
  <div class="planning-summary">
    <strong>Planning</strong>
    {#if task.due_at && task.unplanned_minutes > 0}
      <span class="risk">⚠ {formatMinutesShort(task.unplanned_minutes)} unscheduled before due</span>
    {:else if task.due_at}
      <span class="covered">Schedule coverage {task.schedule_coverage_percent}%</span>
    {/if}
  </div>

  <section>
    <h4>Study blocks</h4>
    {#each blocks as block (block.id)}
      <div class="row">
        <label><input type="date" value={block.planned_date} disabled={busy} onchange={(e) => void run(() => updateStudyBlock({ id: block.id, planned_date: e.currentTarget.value, planned_start_time: block.planned_start_time, planned_minutes: block.planned_minutes, completed: block.completed }))} /></label>
        <label><input type="time" value={block.planned_start_time ?? ""} disabled={busy} onchange={(e) => void run(() => updateStudyBlock({ id: block.id, planned_date: block.planned_date, planned_start_time: e.currentTarget.value || null, planned_minutes: block.planned_minutes, completed: block.completed }))} /></label>
        <label><input type="number" min="1" value={block.planned_minutes} disabled={busy} onchange={(e) => void run(() => updateStudyBlock({ id: block.id, planned_date: block.planned_date, planned_start_time: block.planned_start_time, planned_minutes: Number(e.currentTarget.value), completed: block.completed }))} />m</label>
        <label class="done-label"><input type="checkbox" checked={block.completed} disabled={busy} onchange={(e) => void run(() => updateStudyBlock({ id: block.id, planned_date: block.planned_date, planned_start_time: block.planned_start_time, planned_minutes: block.planned_minutes, completed: e.currentTarget.checked }))} /> Mark block done</label>
        <button class="quiet" disabled={busy} onclick={() => void run(() => deleteStudyBlock(block.id))}>Delete</button>
      </div>
    {/each}
    <form class="add-row" onsubmit={(e) => { e.preventDefault(); if (date && minutes > 0) void run(async () => { await createStudyBlock({ task_id: task.id, planned_date: date, planned_start_time: startTime || null, planned_minutes: minutes }); }); }}>
      <input aria-label="Study block date" required type="date" bind:value={date} />
      <input aria-label="Study block start time" type="time" bind:value={startTime} />
      <input aria-label="Study block minutes" required type="number" min="1" bind:value={minutes} />
      <button type="submit" disabled={busy}>Add block</button>
    </form>
    {#if overlaps(date, startTime || null, minutes)}<p class="overlap-warning">⚠ This Study Block overlaps another planned block. You can still save it.</p>{/if}
  </section>

  <section>
    <h4>Milestones</h4>
    {#each milestones as milestone, index (milestone.id)}
      <div class="milestone" class:done={milestone.completed}>
        <input type="checkbox" checked={milestone.completed} disabled={busy} onchange={(e) => void run(() => updateTaskMilestone({ id: milestone.id, title: milestone.title, target_date: milestone.target_date, completed: e.currentTarget.checked }))} />
        <input class="milestone-title" value={milestone.title} disabled={busy} onchange={(e) => void run(() => updateTaskMilestone({ id: milestone.id, title: e.currentTarget.value, target_date: milestone.target_date, completed: milestone.completed }))} />
        <input type="date" value={milestone.target_date ?? ""} disabled={busy} onchange={(e) => void run(() => updateTaskMilestone({ id: milestone.id, title: milestone.title, target_date: e.currentTarget.value || null, completed: milestone.completed }))} />
        <button class="quiet" disabled={busy || index === 0} onclick={() => void run(() => moveTaskMilestone(milestone.id, "up"))}>↑</button>
        <button class="quiet" disabled={busy || index === milestones.length - 1} onclick={() => void run(() => moveTaskMilestone(milestone.id, "down"))}>↓</button>
        <button class="quiet" disabled={busy} onclick={() => void run(() => deleteTaskMilestone(milestone.id))}>×</button>
      </div>
    {/each}
    <form class="add-row" onsubmit={(e) => { e.preventDefault(); if (milestoneTitle.trim()) void run(async () => { await createTaskMilestone({ task_id: task.id, title: milestoneTitle.trim(), target_date: milestoneDate || null }); milestoneTitle = ""; milestoneDate = ""; }); }}>
      <input aria-label="Milestone title" placeholder="Add milestone" bind:value={milestoneTitle} />
      <input aria-label="Milestone date" type="date" bind:value={milestoneDate} />
      <button type="submit" disabled={busy || !milestoneTitle.trim()}>Add</button>
    </form>
  </section>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

<style>
  .planning { margin: 8px 0 12px 28px; padding: 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--hover); display: grid; gap: 14px; }
  .planning-summary { display: flex; justify-content: space-between; gap: 10px; font-size: 11px; }
  .risk { color: var(--danger); }.covered { color: var(--muted); }
  h4 { margin: 0 0 7px; font-size: 11px; text-transform: uppercase; letter-spacing: .07em; color: var(--muted); }
  .row, .add-row, .milestone { display: flex; align-items: center; gap: 6px; margin-top: 6px; }
  .row input, .add-row input, .milestone input { min-height: 28px; padding: 3px 5px; font-size: 11px; }
  .row input[type=date], .add-row input[type=date], .milestone input[type=date] { width: 125px; }
  .row input[type=time], .add-row input[type=time] { width: 90px; }
  .row input[type=number], .add-row input[type=number] { width: 65px; }
  .milestone-title { flex: 1; min-width: 90px; }
  .add-row button, .quiet { min-height: 28px; padding: 3px 7px; font-size: 11px; }
  .done .milestone-title { text-decoration: line-through; color: var(--muted); }
  .done-label { display:flex; align-items:center; gap:4px; color:var(--muted); font-size:10px; white-space:nowrap; }.overlap-warning { color:#a66a00; font-size:11px; margin:4px 0 0; }
  .error { margin: 0; }
  @media (max-width: 620px) { .row, .add-row, .milestone { flex-wrap: wrap; } }
</style>
