<script lang="ts">
  import { onMount } from "svelte";
  import Modal from "./Modal.svelte";
  import { createTask, listClasses, listTasksForClass, type ClassRecord, type TaskRecord } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";
  let { onclose }: { onclose: () => void } = $props();
  let classes = $state<ClassRecord[]>([]);
  let parents = $state<TaskRecord[]>([]);
  let classId = $state<number | null>(null);
  let parentId = $state<number | null>(null);
  let title = $state("");
  let due = $state("");
  let schedule = $state("today");
  let date = $state("");
  let estimate = $state<number | undefined>();
  let error = $state("");
  let busy = $state(false);
  let parentRequest = 0;
  onMount(() => { void listClasses().then(value => { classes = value; classId = value[0]?.id ?? null; void loadParents(); }).catch(() => error = "Couldn't load classes."); });
  async function loadParents() {
    const request = ++parentRequest; parents = []; parentId = null;
    if (classId === null) return;
    try { const value = await listTasksForClass(classId); if (request === parentRequest) parents = value.filter(t => t.status !== "completed" && t.source === "local"); }
    catch { if (request === parentRequest) error = "Couldn't load parent tasks. Try selecting the class again."; }
  }
  function scheduledDate() {
    if (schedule === "none") return null;
    if (schedule === "date") return date;
    const day = new Date(); if (schedule === "tomorrow") day.setDate(day.getDate() + 1);
    return `${day.getFullYear()}-${String(day.getMonth()+1).padStart(2,"0")}-${String(day.getDate()).padStart(2,"0")}`;
  }
  async function add() {
    if (busy || classId === null) return;
    busy = true; error = "";
    try {
      await createTask({ class_id: classId, parent_task_id: parentId, title: title.trim(), description: null, priority: null, due_at: due || null, scheduled_date: scheduledDate(), estimated_minutes: estimate ?? null });
      timerStore.revision++;
      window.dispatchEvent(new Event("tasks-changed"));
      onclose();
    } catch { error = "Couldn't create the task. Check its details and try again."; }
    finally { busy = false; }
  }
</script>
<Modal title="Quick Add" onclose={() => { if (!busy) onclose(); }}>
  <form onsubmit={e => { e.preventDefault(); void add(); }}>
    <fieldset disabled={busy}>
      <label>Task<input required bind:value={title} /></label>
      <label>Class<select required bind:value={classId} onchange={() => loadParents()}>{#each classes as c}<option value={c.id}>{c.course_code}</option>{/each}</select></label>
      <label>Schedule<select bind:value={schedule}><option value="none">Not scheduled</option><option value="today">Today</option><option value="tomorrow">Tomorrow</option><option value="date">Pick date</option></select></label>
      {#if schedule === "date"}<label>Study date<input required type="date" bind:value={date} /></label>{/if}
      <label>Due date (optional)<input type="date" bind:value={due} /></label>
      <label>Estimate (minutes, optional)<input type="number" min="0" step="1" bind:value={estimate} /></label>
      <label>Parent (optional)<select bind:value={parentId}><option value={null}>None</option>{#each parents as parent}<option value={parent.id}>{parent.title}</option>{/each}</select></label>
      {#if classes.length === 0}<p>Create a class in Classes first.</p>{/if}
      <button disabled={!title.trim() || classId === null} type="submit">{busy ? "Adding…" : "Add"}</button>
    </fieldset>
    {#if error}<p role="alert">{error}</p>{/if}
  </form>
</Modal>
<style>
fieldset { border: 0; padding: 0; margin: 0; display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  label { display: block; margin: 0; }
  label:first-child, label:last-of-type { grid-column: 1 / -1; }
  input, select { display: block; width: 100%; }
  button { grid-column: 1 / -1; margin-top: 10px; }
  fieldset p { grid-column: 1 / -1; font-size: 12px; color: var(--muted); }
</style>
