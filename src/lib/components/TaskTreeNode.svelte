<script lang="ts">
  import TaskTreeNode from "./TaskTreeNode.svelte";
  import { completeTodoistTask, duplicateTask, scheduleTask, setTaskStatus, updateTask, type TaskRecord, type TaskType } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatDurationShort, localDate } from "$lib/format";
  import Icon from "./Icon.svelte";
  import TaskPlanning from "./TaskPlanning.svelte";
  import { PRIORITY_LABELS, DEFAULT_PRIORITY } from "$lib/priority";
  import { TASK_TYPES, parseTags, taskTypeLabel } from "$lib/taskMetadata";
  type TreeTask = TaskRecord & { context_only?: boolean; overdue?: boolean };
  let { task, tasks, courseCode, onchanged, onhistory, ondelete, onedit, path = [] }: {
    task: TreeTask; tasks: TreeTask[]; courseCode: string; onchanged: () => Promise<void>;
    onhistory?: (id: number) => void; ondelete?: (id: number) => void; onedit?: (task: TaskRecord) => void; path?: number[];
  } = $props();
  let busy = $state(false);
  let error = $state("");
  let pickingDate = $state(false);
  let editing = $state(false);
  let editTitle = $state("");
  let editDue = $state("");
  let editEstimate = $state("");
  let editPriority = $state(DEFAULT_PRIORITY);
  let editType = $state<TaskType>("assignment");
  let editTags = $state("");
  let editBudget = $state("");
  let undoStatus = $state<TaskRecord["status"] | null>(null);
  let planningOpen = $state(false);
  let menu: HTMLDetailsElement;
  let children = $derived(tasks.filter(t => t.parent_task_id === task.id && !path.includes(t.id) && t.id !== task.id));
  async function change(action: () => Promise<unknown>) {
    if (busy) return false;
    busy = true; error = "";
    try { await action(); await onchanged(); if (menu) menu.open = false; return true; }
    catch { error = "Couldn't save this task change. Try again."; return false; }
    finally { busy = false; }
  }
  async function saveEdit() {
    if (!editTitle.trim()) return;
    if (await change(() => updateTask({ id: task.id, title: editTitle.trim(), description: task.description, priority: editPriority, due_at: editDue || null, scheduled_date: task.scheduled_date, estimated_minutes: editEstimate ? Number(editEstimate) : null, task_type: editType, tags: parseTags(editTags), time_budget_minutes: editBudget ? Number(editBudget) : null }))) editing = false;
  }
  async function complete() {
    if (task.source === "todoist") await change(() => completeTodoistTask(task.id));
    else {
      const previous = task.status;
      if (await change(() => setTaskStatus(task.id, task.status === "completed" ? "not_started" : "completed"))) {
        undoStatus = previous;
        window.setTimeout(() => { undoStatus = null; }, 5000);
      }
    }
  }
  function tomorrow() { const d = new Date(); d.setDate(d.getDate() + 1); return localDate(d); }
</script>
<li>
  <div class="task-row" class:context={task.context_only} class:tracking={timerStore.active?.session.task_id === task.id}>
    <input type="checkbox" aria-label={`Complete ${task.title}`} checked={task.status === "completed"}
      disabled={busy || (task.source === "todoist" && task.status === "completed")}
      title={task.source === "todoist" ? "Complete in Todoist" : "Complete task"}
      onchange={complete} />
    <span class="title" class:done={task.status === "completed"}>{task.title}
      <span class="badges">
      {#if task.source === "todoist"}<small title="Complete and edit deadlines in Todoist, then sync">Todoist</small>{/if}
      {#if task.external_state !== "active"}<small>{task.external_state}</small>{/if}
      {#if task.overdue}<small class="overdue">Overdue</small>{:else if task.due_at}<small>Due {task.due_at.slice(5, 10).replace("-", "/")}</small>{/if}
      {#if task.context_only}<small>Parent task</small>{/if}
      <small>{taskTypeLabel(task.task_type)}</small>
      {#each task.tags as tag}<small>#{tag}</small>{/each}
      {#if task.time_budget_minutes !== null}<small>{task.time_budget_minutes}m budget</small>{/if}
      </span>
    </span>
    <span class="meta" title="Estimated direct work">{task.estimated_minutes !== null ? `${task.estimated_minutes}m` : ""}<small>{task.estimated_minutes !== null ? "est." : ""}</small></span>
    <span class="meta" title="Tracked time including subtasks">{task.tracked_seconds > 0 ? formatDurationShort(task.tracked_seconds) : ""}<small>{task.tracked_seconds > 0 ? "tracked" : ""}</small></span>
    {#if task.status !== "completed" && task.external_state === "active"}
      <button disabled={timerStore.busy || timerStore.active?.session.task_id === task.id}
        class="start" aria-label={timerStore.active?.session.task_id === task.id ? `Tracking ${task.title}` : `Start ${task.title}`} title={timerStore.active?.session.task_id === task.id ? "Currently tracking" : "Start timer"}
        onclick={() => timerStore.start(task.id, `${courseCode} — ${task.title}`)}>
        <Icon name={timerStore.active?.session.task_id === task.id ? "clock" : "play"} size={15} />
      </button>
    {/if}
    <details bind:this={menu}>
      <summary aria-label={`Actions for ${task.title}`}>•••</summary>
      <div class="actions">
        {#if task.due_at}<small>Due: {task.due_at.slice(0, 10)}</small>{/if}
        {#if task.scheduled_date}<small>Scheduled: {task.scheduled_date}</small>{/if}
        <button disabled={busy} onclick={() => change(() => scheduleTask(task.id, localDate()))}>Do today</button>
        <button disabled={busy} onclick={() => change(() => scheduleTask(task.id, tomorrow()))}>Schedule tomorrow</button>
        <button onclick={() => pickingDate = !pickingDate}>Pick date</button>
        {#if pickingDate}<input aria-label={`Schedule ${task.title}`} type="date" value={task.scheduled_date ?? ""} disabled={busy}
          onchange={(e) => change(() => scheduleTask(task.id, e.currentTarget.value || null))} />{/if}
        {#if task.scheduled_date}<button disabled={busy} onclick={() => change(() => scheduleTask(task.id, null))}>Clear schedule</button>{/if}
        {#if task.source === "local"}<button onclick={() => { editing = !editing; menu.open = false; editTitle = task.title; editDue = task.due_at?.slice(0, 10) ?? ""; editEstimate = task.estimated_minutes?.toString() ?? ""; editPriority = task.priority ?? DEFAULT_PRIORITY; editType = task.task_type; editTags = task.tags.join(", "); editBudget = task.time_budget_minutes?.toString() ?? ""; }}>Edit task</button>{/if}
        <button onclick={() => { planningOpen = !planningOpen; menu.open = false; }}>Plan blocks &amp; milestones</button>
        {#if task.source === "local"}<button disabled={busy} onclick={() => change(() => duplicateTask(task.id))}>Duplicate</button>{/if}
        {#if onhistory}<button onclick={() => onhistory?.(task.id)}>History</button>{/if}
        {#if ondelete && task.source === "local"}<button onclick={() => ondelete?.(task.id)}>Delete</button>{/if}
      </div>
    </details>
  </div>
  {#if editing}
    <form class="edit-form" onsubmit={(event) => { event.preventDefault(); void saveEdit(); }}>
      <label>Task title<input required bind:value={editTitle} /></label>
      <label>Due date<input type="date" bind:value={editDue} /></label>
      <label>Estimate (min)<input type="number" min="0" bind:value={editEstimate} /></label>
      <label>Type<select bind:value={editType}>{#each TASK_TYPES as type}<option value={type.value}>{type.label}</option>{/each}</select></label>
      <label>Time budget (min)<input type="number" min="0" bind:value={editBudget} /></label>
      <label>Tags<input placeholder="reading, exam" bind:value={editTags} /></label>
      <label>Priority
        <select bind:value={editPriority}>
          {#each Object.entries(PRIORITY_LABELS) as [value, label] (value)}
            <option value={Number(value)}>{label}</option>
          {/each}
        </select>
      </label>
      <div class="edit-buttons"><button type="submit" disabled={busy}>Save changes</button><button type="button" disabled={busy} onclick={() => editing = false}>Cancel</button></div>
    </form>
  {/if}
  {#if undoStatus && task.status !== undoStatus}<button class="undo" disabled={busy} onclick={() => change(() => setTaskStatus(task.id, undoStatus ?? "not_started")).then(ok => { if (ok) undoStatus = null; })}>Undo completion</button>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
  {#if planningOpen}<TaskPlanning {task} onchanged={onchanged} />{/if}
  {#if children.length}
    <ul>
      {#each children as child (child.id)}
        <TaskTreeNode task={child} {tasks} {courseCode} {onchanged} {onhistory} {ondelete} {onedit} path={[...path, task.id]} />
      {/each}
    </ul>
  {/if}
</li>
<style>
li { list-style: none; min-width: 0; }
  ul { padding-left: 18px; margin: 0 0 0 7px; border-left: 1px solid var(--line); }
  .task-row { display: flex; gap: 10px; align-items: center; min-height: 54px; padding: 7px 4px; border-bottom: 1px solid var(--line); }
  .task-row.tracking { background: var(--accent-soft); border-radius: 6px; padding-inline: 7px; }
  .title { flex: 1; min-width: 0; overflow-wrap: anywhere; font-size: 13px; line-height: 1.4; }
  .done { text-decoration: line-through; color: var(--muted); }
  .badges { display: flex; gap: 7px; flex-wrap: wrap; }
  small { font-size: 10px; color: var(--muted); font-weight: 400; }
  small.overdue { color: var(--danger); }
  .meta { min-width: 40px; text-align: right; font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .meta small { display: block; font-size: 9px; }
  .start { padding: 0; min-height: 29px; width: 29px; color: var(--accent); border-color: transparent; background: var(--accent-soft); flex-shrink: 0; }
  summary { cursor: pointer; list-style: none; color: var(--muted); width: 24px; text-align: center; padding: 5px 0; border-radius: 4px; }
  summary::-webkit-details-marker { display: none; }
  summary:hover { background: var(--hover); }
  details { position: relative; }
  .actions { position: absolute; z-index: 5; right: 0; width: 205px; padding: 7px; background: var(--surface); border: 1px solid var(--line); border-radius: 8px; display: grid; gap: 3px; box-shadow: var(--shadow); }
  .actions button { border: 0; background: transparent; justify-content: start; font-size: 12px; }
  .actions small { padding: 3px 8px; }
  .context > .title { color: var(--muted); }
  .edit-form { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; padding: 15px; background: var(--hover); border-radius: 8px; margin-block: 8px; }
  .edit-form label { display: grid; gap: 4px; font-size: 11px; color: var(--muted); }
  .edit-form label:first-child, .edit-buttons { grid-column: 1 / -1; }
  .edit-buttons { display: flex; gap: 8px; }
  @media(max-width: 550px) { .task-row { gap: 6px; } .meta { min-width: 28px; font-size: 10px; } ul { padding-left: 12px; } }
</style>
