<script lang="ts">
  import TaskTreeNode from "./TaskTreeNode.svelte";
  import { scheduleTask, setTaskStatus, updateTask, type TaskRecord } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatDurationShort, localDate } from "$lib/format";
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
  let children = $derived(tasks.filter(t => t.parent_task_id === task.id && !path.includes(t.id) && t.id !== task.id));
  async function change(action: () => Promise<unknown>) {
    busy = true; error = "";
    try { await action(); await onchanged(); }
    catch { error = "Couldn't save this task change. Try again."; }
    finally { busy = false; }
  }
  function tomorrow() { const d = new Date(); d.setDate(d.getDate() + 1); return localDate(d); }
</script>
<li>
  <div class="task-row" class:context={task.context_only}>
    <input type="checkbox" aria-label={`Complete ${task.title}`} checked={task.status === "completed"}
      disabled={busy || task.source === "todoist"}
      title={task.source === "todoist" ? "Complete in Todoist, then sync" : "Complete task"}
      onchange={() => change(() => setTaskStatus(task.id, task.status === "completed" ? "not_started" : "completed"))} />
    <span class="title" class:done={task.status === "completed"}>{task.title}
      {#if task.source === "todoist"}<small title="Completion and due dates are controlled by Todoist">Todoist · read-only</small>{/if}
      {#if task.external_state !== "active"}<small>{task.external_state}</small>{/if}
      {#if task.context_only}<small>context</small>{/if}
      {#if task.overdue}<small>overdue</small>{/if}
    </span>
    {#if task.estimated_minutes !== null}<span class="meta">{task.estimated_minutes}m est.</span>{/if}
    {#if task.tracked_seconds > 0}<span class="meta" title="This task and its descendants">{formatDurationShort(task.tracked_seconds)}</span>{/if}
    {#if task.status !== "completed" && task.external_state === "active"}
      <button disabled={timerStore.busy || timerStore.active?.session.task_id === task.id}
        onclick={() => timerStore.start(task.id, `${courseCode} — ${task.title}`)}>
        {timerStore.active?.session.task_id === task.id ? "Tracking" : "Start"}
      </button>
    {/if}
    <details>
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
        {#if task.source === "local"}<button onclick={() => { editing = !editing; editTitle = task.title; editDue = task.due_at?.slice(0, 10) ?? ""; editEstimate = task.estimated_minutes?.toString() ?? ""; }}>Edit</button>{/if}
        {#if onhistory}<button onclick={() => onhistory?.(task.id)}>History</button>{/if}
        {#if ondelete && task.source === "local"}<button onclick={() => ondelete?.(task.id)}>Delete</button>{/if}
      </div>
    </details>
  </div>
  {#if editing}
    <form class="edit-form" onsubmit={(event) => { event.preventDefault(); void change(() => updateTask({ id: task.id, title: editTitle, description: task.description, priority: task.priority, due_at: editDue || null, scheduled_date: task.scheduled_date, estimated_minutes: editEstimate ? Number(editEstimate) : null })); editing = false; }}>
      <input aria-label="Task title" bind:value={editTitle} />
      <input aria-label="Due date" type="date" bind:value={editDue} />
      <input aria-label="Estimated minutes" type="number" min="0" bind:value={editEstimate} />
      <button type="submit">Save</button>
    </form>
  {/if}
  {#if error}<p role="alert">{error}</p>{/if}
  {#if children.length}
    <ul>
      {#each children as child (child.id)}
        <TaskTreeNode task={child} {tasks} {courseCode} {onchanged} {onhistory} {ondelete} {onedit} path={[...path, task.id]} />
      {/each}
    </ul>
  {/if}
</li>
<style>
  li { list-style: none; }
  ul { padding-left: 1.25rem; margin: 0; border-left: 1px solid #aaa4; }
  .task-row { display: flex; gap: .55rem; align-items: center; min-height: 2.7rem; border-bottom: 1px solid #aaa3; }
  .title { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .done { text-decoration: line-through; opacity: .65; }
  small { display: block; font-size: .72rem; opacity: .7; }
  .meta { font-size: .8rem; white-space: nowrap; opacity: .7; }
  button { cursor: pointer; border: 1px solid #aaa5; border-radius: 5px; padding: .3rem .5rem; color: inherit; background: transparent; }
  button:disabled { cursor: default; opacity: .5; }
  summary { cursor: pointer; list-style: none; padding: .4rem; }
  details { position: relative; }
  .actions { position: absolute; z-index: 5; right: 0; width: 170px; padding: .65rem; background: #fafafa; border: 1px solid #aaa; border-radius: 6px; display: grid; gap: .4rem; box-shadow: 0 3px 10px #0002; }
  .context > .title { opacity: .7; }
  .edit-form { display: flex; gap: .4rem; padding: .4rem 0 .4rem 1.8rem; }
  @media(prefers-color-scheme: dark) { .actions { background: #303030; } }
</style>
