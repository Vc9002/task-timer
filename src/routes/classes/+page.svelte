<script lang="ts">
  import { onMount } from "svelte";
  import TaskTreeNode from "$lib/components/TaskTreeNode.svelte";
  import {
    listClasses,
    createClass,
    updateClass,
    archiveClass,
    listTasksForClass,
    createTask,
    deleteTask,
    getTaskHistory,
    type ClassRecord,
    type TaskRecord,
    type TaskHistory,
  } from "$lib/api";
  import { formatDurationShort, formatSignedDurationShort } from "$lib/format";

  let classes = $state<ClassRecord[]>([]);
  let tasksByClass = $state<Record<number, TaskRecord[]>>({});
  let expanded = $state<number | null>(null);
  let editingClassId = $state<number | null>(null);
  let editCourseCode = $state("");
  let editName = $state("");
  let editSemester = $state("");

  let courseCode = $state("");
  let name = $state("");
  let semester = $state("");
  let error = $state("");

  // Quick task-add form state, keyed loosely per expanded class.
  let taskTitle = $state("");
  let taskDue = $state("");
  let taskEstimate = $state("");
  let taskParentId = $state<number | null>(null);

  let historyTaskId = $state<number | null>(null);
  let history = $state<TaskHistory | null>(null);

  async function refresh() {
    try {
      classes = await listClasses();
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function addClass(event: Event) {
    event.preventDefault();
    if (!courseCode.trim() || !semester.trim()) return;
    try {
      await createClass({
        course_code: courseCode.trim(),
        name: name.trim() || null,
        semester: semester.trim(),
        color: null,
      });
      courseCode = "";
      name = "";
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(id: number) {
    try {
      await archiveClass(id);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  function startEditClass(c: ClassRecord) {
    editingClassId = c.id;
    editCourseCode = c.course_code;
    editName = c.name ?? "";
    editSemester = c.semester;
  }

  async function saveEditClass(c: ClassRecord) {
    try {
      await updateClass({
        id: c.id,
        course_code: editCourseCode.trim(),
        name: editName.trim() || null,
        semester: editSemester.trim(),
        color: c.color,
        active: c.active,
      });
      editingClassId = null;
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleExpand(classId: number) {
    if (expanded === classId) {
      expanded = null;
      return;
    }
    expanded = classId;
    taskParentId = null;
    try {
      tasksByClass[classId] = await listTasksForClass(classId);
    } catch (e) {
      error = String(e);
    }
  }

  async function addTask(classId: number, event: Event) {
    event.preventDefault();
    if (!taskTitle.trim()) return;
    try {
      await createTask({
        class_id: classId,
        parent_task_id: taskParentId,
        title: taskTitle.trim(),
        description: null,
        priority: null,
        due_at: taskDue || null,
        scheduled_date: null,
        estimated_minutes: taskEstimate ? Number(taskEstimate) : null,
      });
      taskTitle = "";
      taskDue = "";
      taskEstimate = "";
      taskParentId = null;
      tasksByClass[classId] = await listTasksForClass(classId);
    } catch (e) {
      error = String(e);
    }
  }

  async function removeTask(classId: number, id: number) {
    try {
      await deleteTask(id);
      tasksByClass[classId] = await listTasksForClass(classId);
    } catch (e) {
      error = String(e);
    }
  }

  async function toggleHistory(taskId: number) {
    if (historyTaskId === taskId) {
      historyTaskId = null;
      history = null;
      return;
    }
    historyTaskId = taskId;
    try {
      history = await getTaskHistory(taskId);
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    void refresh();
    const reload = () => { const id = expanded; if (id !== null) void listTasksForClass(id).then(tasks => { tasksByClass[id] = tasks; }).catch(() => error = "Couldn't refresh tasks. Try reopening the class."); };
    window.addEventListener("tasks-changed", reload);
    return () => window.removeEventListener("tasks-changed", reload);
  });
</script>


<main class="container">
  <h1>Classes</h1>
  <p class="page-intro">Keep coursework and subtasks together.</p>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <form class="row" onsubmit={addClass}>
    <input aria-label="Course code" required placeholder="Course code · LGST 1000" bind:value={courseCode} />
    <input aria-label="Class name" placeholder="Class name (optional)" bind:value={name} />
    <input aria-label="Semester" required placeholder="Semester · Fall 2026" bind:value={semester} />
    <button type="submit">Add class</button>
  </form>

  <ul class="class-list">
    {#each classes as c (c.id)}
      <li>
        {#if editingClassId === c.id}
          <div class="class-header editing">
            <input bind:value={editCourseCode} placeholder="Course code" />
            <input bind:value={editName} placeholder="Full name (optional)" />
            <input bind:value={editSemester} placeholder="Semester" />
            <button onclick={() => saveEditClass(c)}>Save</button>
            <button class="secondary" onclick={() => (editingClassId = null)}>Cancel</button>
          </div>
        {:else}
          <div class="class-header">
            <button class="secondary" aria-expanded={expanded === c.id} onclick={() => toggleExpand(c.id)}>Tasks</button>
            <span class="code">{c.course_code}</span>
            {#if c.name}<span class="name">{c.name}</span>{/if}
            <span class="semester">{c.semester}</span>
            <button class="secondary" onclick={(e) => { e.stopPropagation(); startEditClass(c); }}>Edit</button>
            <button class="secondary" onclick={(e) => { e.stopPropagation(); remove(c.id); }}>Archive</button>
          </div>
        {/if}

        {#if expanded === c.id}
          <div class="tasks">
            <form class="row" onsubmit={(e) => addTask(c.id, e)}>
              <input aria-label="Task title" required placeholder="Add a task…" bind:value={taskTitle} />
              <input aria-label="Due date" type="date" bind:value={taskDue} />
              <input aria-label="Estimated minutes" placeholder="Est. minutes" type="number" min="0" bind:value={taskEstimate} />
              <select aria-label="Parent task" bind:value={taskParentId}>
                <option value={null}>No parent</option>
                {#each (tasksByClass[c.id] ?? []).filter((t) => t.source === "local" && t.status !== "completed") as parent}
                  <option value={parent.id}>{parent.title}</option>
                {/each}
              </select>
              <button type="submit">Add task</button>
            </form>

            <ul class="task-list">
              {#each (tasksByClass[c.id] ?? []).filter(t => !(tasksByClass[c.id] ?? []).some(p => p.id === t.parent_task_id)) as t (t.id)}
                <TaskTreeNode task={t} tasks={tasksByClass[c.id] ?? []} courseCode={c.course_code}
                  onchanged={async () => { tasksByClass[c.id] = await listTasksForClass(c.id); }}
                  onhistory={toggleHistory} ondelete={(id) => removeTask(c.id, id)} />
              {/each}
                {#if historyTaskId !== null && history}
                  <li class="history-panel">
                    <div class="history-summary">
                      <span>Total: {formatDurationShort(history.total_seconds)}</span>
                      {#if history.estimated_minutes}
                        <span>Estimate: {formatDurationShort(history.estimated_minutes * 60)}</span>
                        <span>
                          Difference: {formatSignedDurationShort(
                            history.total_seconds - history.estimated_minutes * 60
                          )}
                        </span>
                      {/if}
                    </div>
                    <ul class="history-sessions">
                      {#each history.sessions as s (s.session_id)}
                        <li>
                          <span>{new Date(s.start_ts.replace(" ", "T") + "Z").toLocaleDateString()}</span>
                          <span>{formatDurationShort(s.duration_seconds)}</span>
                        </li>
                      {/each}
                      {#if history.sessions.length === 0}
                        <li class="empty">No sessions yet.</li>
                      {/if}
                    </ul>
                  </li>
                {/if}
              {#if (tasksByClass[c.id] ?? []).length === 0}
                <li class="empty">No tasks yet.</li>
              {/if}
            </ul>
          </div>
        {/if}
      </li>
    {/each}
    {#if classes.length === 0}
      <li class="empty">No classes yet — add one above.</li>
    {/if}
  </ul>
</main>

<style>
.row { display: flex; flex-wrap: wrap; gap: 8px; margin: 16px 0; }
  .row input, .row select { flex: 1; min-width: 115px; }
  .class-list { list-style: none; padding: 0; display: grid; gap: 16px; margin-top: 25px; }
  .class-list > li:not(.empty) { border: 1px solid var(--line); border-radius: 8px; background: var(--surface); }
  .class-header { display: flex; align-items: center; flex-wrap: wrap; gap: 9px; padding: 14px; }
  .class-header .code { font-size: 12px; font-weight: 650; }
  .class-header .name { color: var(--muted); font-size: 12px; flex: 1; }
  .semester { color: var(--muted); font-size: 11px; margin-left: auto; }
  .class-header button { padding: 3px 8px; min-height: 26px; font-size: 11px; }
  .class-header.editing input { min-width: 100px; flex: 1; }
  .tasks { padding: 0 16px 16px; border-top: 1px solid var(--line); }
  .task-list { padding: 0; list-style: none; }
  .history-panel { padding: 12px; margin-top: 12px; background: var(--hover); border-radius: 6px; }
  .history-summary { display: flex; flex-wrap: wrap; gap: 14px; color: var(--muted); font-size: 12px; }
  .history-sessions { list-style: none; padding: 0; }
  .history-sessions li { display: flex; justify-content: space-between; font-size: 12px; padding-top: 6px; }
</style>
