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
    const reload = () => { if (expanded !== null) void listTasksForClass(expanded).then(tasks => { if (expanded !== null) tasksByClass[expanded] = tasks; }); };
    window.addEventListener("tasks-changed", reload);
    return () => window.removeEventListener("tasks-changed", reload);
  });
</script>


<main class="container">
  <h1>Classes</h1>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <form class="row" onsubmit={addClass}>
    <input placeholder="Course code (e.g. LGST 1000)" bind:value={courseCode} />
    <input placeholder="Full name (optional)" bind:value={name} />
    <input placeholder="Semester (e.g. Fall 2026)" bind:value={semester} />
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
              <input placeholder="Task title" bind:value={taskTitle} />
              <input type="date" bind:value={taskDue} />
              <input placeholder="Est. minutes" type="number" bind:value={taskEstimate} />
              <select bind:value={taskParentId}>
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
  .container {
    margin: 0 auto;
    max-width: 720px;
    padding: 3rem 1.5rem;
  }

  h1 {
    margin-bottom: 1.5rem;
  }

  .row {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
    flex-wrap: wrap;
  }

  input,
  select {
    flex: 1;
    min-width: 100px;
    padding: 0.5em 0.75em;
    border-radius: 6px;
    border: 1px solid #ccc;
  }

  button {
    padding: 0.5em 1em;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #396cd8;
    color: white;
    cursor: pointer;
  }

  .class-list {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .class-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6em 0.9em;
    background: white;
    border-radius: 6px;
    border: 1px solid #e5e5e5;
    cursor: pointer;
  }

  .class-header button {
    margin-left: auto;
    background: #e5e5e5;
    color: #333;
  }

  .class-header button + button {
    margin-left: 0;
  }

  .class-header.editing {
    cursor: default;
  }

  button.secondary {
    background: #e5e5e5;
    color: #333;
  }

  .history-panel {
    margin-left: 1.5rem;
    padding: 0.5rem 0.75rem;
    background: white;
    border-radius: 6px;
    border: 1px dashed #ccc;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .history-summary {
    display: flex;
    gap: 1rem;
    font-size: 0.85em;
    color: #555;
  }

  .history-sessions {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .history-sessions li {
    display: flex;
    justify-content: space-between;
    font-size: 0.85em;
    color: #777;
  }

  .code {
    font-weight: 600;
  }

  .semester {
    color: #666;
    font-size: 0.9em;
  }

  .tasks {
    padding: 0.75rem 1rem;
    background: #fafafa;
    border: 1px solid #ececec;
    border-top: none;
    border-radius: 0 0 6px 6px;
  }

  .task-list {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .task-list li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.35em 0.4em;
  }

  .empty {
    color: #888;
    font-style: italic;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    .class-header,
    .tasks,
    .history-panel {
      background: #3a3a3a;
      border-color: #4a4a4a;
    }
    input,
    select {
      background: #2a2a2a;
      color: #f6f6f6;
      border-color: #555;
    }
  }
</style>
