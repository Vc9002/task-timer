<script lang="ts">
  import { onMount } from "svelte";
  import Modal from "./Modal.svelte";
  import { createTask, getEstimateSuggestion, instantiateTaskTemplate, listClasses, listTaskTemplates, listTasksForClass, type ClassRecord, type EstimateSuggestion, type TaskRecord, type TaskTemplate, type TaskType } from "$lib/api";
  import { TASK_TYPES, parseTags } from "$lib/taskMetadata";
  import { timerStore } from "$lib/stores/timer.svelte";
  let { onclose }: { onclose: () => void } = $props();
  let classes = $state<ClassRecord[]>([]);
  let parents = $state<TaskRecord[]>([]);
  let classId = $state<number | null>(null);
  let parentId = $state<number | null>(null);
  let title = $state("");
  let shorthand = $state("");
  let due = $state("");
  let schedule = $state("today");
  let date = $state("");
  let estimate = $state<number | undefined>();
  let budget = $state<number | undefined>();
  let taskType = $state<TaskType>("assignment");
  let tags = $state("");
  let templates = $state<TaskTemplate[]>([]);
  let templateId = $state<number | null>(null);
  let error = $state("");
  let busy = $state(false);
  let suggestion = $state<EstimateSuggestion | null>(null);
  let suggestionRequest = 0;
  let parentRequest = 0;
  onMount(() => {
    void Promise.all([listClasses(), listTaskTemplates()]).then(([classValue, templateValue]) => { classes = classValue; templates = templateValue; classId = classValue[0]?.id ?? null; void loadParents(); }).catch(() => error = "Couldn't load classes.");
  });
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
  function parseShorthand() {
    const raw = shorthand.trim(); if (!raw) return;
    const lower = raw.toLowerCase();
    const matchedClass = classes.find(item => lower.includes(item.course_code.toLowerCase()));
    if (matchedClass) { classId = matchedClass.id; void loadParents(); }
    const duration = raw.match(/(\d+)\s*m(?:in(?:ute)?s?)?/i); if (duration) estimate = Number(duration[1]);
    const relative = lower.includes("tomorrow") ? 1 : lower.includes("today") ? 0 : null;
    if (relative !== null) { schedule = relative === 0 ? "today" : "tomorrow"; }
    const dueMatch = lower.match(/\bdue\s+(today|tomorrow|mon|tue|wed|thu|fri|sat|sun)/);
    if (dueMatch) { due = dateFromToken(dueMatch[1]); }
    const removable = new RegExp(`\\b(?:${classes.map(item => item.course_code.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\\\$&")).join("|")}|\\d+\\s*m(?:in(?:ute)?s?)?|today|tomorrow|due\\s+(?:today|tomorrow|mon|tue|wed|thu|fri|sat|sun))\\b`, "gi");
    title = raw.replace(removable, "").replace(/\s+/g, " ").trim(); shorthand = "";
  }
  function dateFromToken(token: string): string { const now = new Date(); if (token === "today") return scheduledDateFor(now); if (token === "tomorrow") { now.setDate(now.getDate() + 1); return scheduledDateFor(now); } const names = ["sun","mon","tue","wed","thu","fri","sat"]; const target = names.indexOf(token.slice(0,3)); if (target >= 0) { const delta = (target - now.getDay() + 7) % 7 || 7; now.setDate(now.getDate() + delta); } return scheduledDateFor(now); }
  function scheduledDateFor(value: Date) { return `${value.getFullYear()}-${String(value.getMonth()+1).padStart(2,"0")}-${String(value.getDate()).padStart(2,"0")}`; }
  async function loadSuggestion() {
    const request = ++suggestionRequest;
    if (classId === null) { suggestion = null; return; }
    try { const value = await getEstimateSuggestion(classId, taskType); if (request === suggestionRequest) suggestion = value; }
    catch { if (request === suggestionRequest) suggestion = null; }
  }
  $effect(() => { void classId; void taskType; void loadSuggestion(); });
  async function add() {
    if (busy || classId === null) return;
    busy = true; error = "";
    try {
      if (templateId !== null) {
        await instantiateTaskTemplate({ template_id: templateId, class_id: classId, title: title.trim(), due_at: due || null, scheduled_date: scheduledDate() });
      } else {
        await createTask({ class_id: classId, parent_task_id: parentId, title: title.trim(), description: null, priority: null, due_at: due || null, scheduled_date: scheduledDate(), estimated_minutes: estimate ?? null, task_type: taskType, tags: parseTags(tags), time_budget_minutes: budget ?? null });
      }
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
      <label class="shorthand">Shorthand (optional)<div class="shorthand-row"><input placeholder="LGST case brief due Fri 60m tomorrow" bind:value={shorthand} /><button type="button" class="secondary" onclick={parseShorthand}>Parse</button></div></label>
      <label>Task<input required bind:value={title} /></label>
      {#if templates.length}<label>Template<select bind:value={templateId} onchange={() => { const template = templates.find(item => item.id === templateId); if (template) { taskType = template.task_type ?? "assignment"; estimate = template.default_estimated_minutes ?? undefined; budget = template.default_time_budget_minutes ?? undefined; tags = template.tags.join(", "); if (template.class_id) { classId = template.class_id; void loadParents(); } } }}><option value={null}>No template</option>{#each templates as template}<option value={template.id}>{template.name}</option>{/each}</select></label>{/if}
      <label>Class<select required bind:value={classId} onchange={() => loadParents()}>{#each classes as c}<option value={c.id}>{c.course_code}</option>{/each}</select></label>
      <label>Schedule<select bind:value={schedule}><option value="none">Not scheduled</option><option value="today">Today</option><option value="tomorrow">Tomorrow</option><option value="date">Pick date</option></select></label>
      {#if schedule === "date"}<label>Study date<input required type="date" bind:value={date} /></label>{/if}
      <label>Due date (optional)<input type="date" bind:value={due} /></label>
      <label>Estimate (minutes, optional)<input type="number" min="0" step="1" bind:value={estimate} /></label>
      {#if suggestion?.suggested_minutes}<p class="suggestion">Based on {suggestion.sample_count} similar completed tasks: <strong>{suggestion.suggested_minutes}m suggested</strong> <button type="button" class="quiet" onclick={() => estimate = suggestion?.suggested_minutes ?? estimate}>Use suggestion</button></p>{/if}
      <label>Type<select bind:value={taskType}>{#each TASK_TYPES as type}<option value={type.value}>{type.label}</option>{/each}</select></label>
      <label>Time budget (minutes, optional)<input type="number" min="0" step="1" bind:value={budget} /></label>
      <label>Tags (comma-separated)<input placeholder="reading, exam" bind:value={tags} /></label>
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
  label:first-child, label:last-of-type, .shorthand { grid-column: 1 / -1; }
  .shorthand-row { display:flex; gap:6px; margin-top:4px; }.shorthand-row input { flex:1; }.shorthand-row button { margin:0; }
  input, select { display: block; width: 100%; }
  button { grid-column: 1 / -1; margin-top: 10px; }
  fieldset p { grid-column: 1 / -1; font-size: 12px; color: var(--muted); }
  .suggestion { margin: -4px 0 0; padding: 8px 10px; border-left: 3px solid var(--accent); background: var(--accent-soft); }
  .suggestion button { display: inline; min-height: 0; padding: 0; margin-left: 5px; color: var(--accent); }
</style>
