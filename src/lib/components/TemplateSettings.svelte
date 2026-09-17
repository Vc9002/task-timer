<script lang="ts">
  import { onMount } from "svelte";
  import { createTaskTemplate, createTemplateMilestone, deleteTaskTemplate, deleteTemplateMilestone, listClasses, listTaskTemplates, listTemplateMilestones, type ClassRecord, type TaskTemplate, type TaskType, type TemplateMilestone } from "$lib/api";
  import { TASK_TYPES, parseTags } from "$lib/taskMetadata";
  let templates = $state<TaskTemplate[]>([]);
  let classes = $state<ClassRecord[]>([]);
  let name = $state("");
  let classId = $state<number | null>(null);
  let taskType = $state<TaskType>("assignment");
  let estimate = $state<number | undefined>();
  let budget = $state<number | undefined>();
  let tags = $state("");
  let error = $state("");
  let busy = $state(false);
  let selectedTemplate = $state<number | null>(null);
  let milestones = $state<TemplateMilestone[]>([]);
  let milestoneTitle = $state("");
  let milestoneOffset = $state<number | undefined>();
  async function refresh() {
    try { [templates, classes] = await Promise.all([listTaskTemplates(), listClasses()]); error = ""; }
    catch { error = "Couldn't load templates."; }
  }
  onMount(() => { void refresh(); });
  async function selectTemplate(id: number) { selectedTemplate = id; try { milestones = await listTemplateMilestones(id); } catch { error = "Couldn't load template milestones."; } }
  async function addMilestone() { if (!selectedTemplate || !milestoneTitle.trim() || busy) return; busy = true; try { await createTemplateMilestone({ template_id: selectedTemplate, title: milestoneTitle.trim(), offset_days_before_due: milestoneOffset ?? null }); milestoneTitle = ""; milestoneOffset = undefined; await selectTemplate(selectedTemplate); } catch { error = "Couldn't save the template milestone."; } finally { busy = false; } }
  async function add() {
    if (!name.trim() || busy) return;
    busy = true; error = "";
    try { await createTaskTemplate({ name: name.trim(), class_id: classId, task_type: taskType, default_estimated_minutes: estimate ?? null, default_time_budget_minutes: budget ?? null, priority: null, tags: parseTags(tags) }); name = ""; estimate = undefined; budget = undefined; tags = ""; await refresh(); }
    catch { error = "Couldn't save this template."; }
    finally { busy = false; }
  }
</script>

<section class="settings-section">
  <h2>Templates</h2>
  <p class="hint">Create reusable coursework patterns. Use them from Quick Add; the template never silently creates a task.</p>
  <form class="template-form" onsubmit={(e) => { e.preventDefault(); void add(); }}>
    <input required placeholder="Template name · Case Brief" bind:value={name} />
    <select bind:value={classId}><option value={null}>Any class</option>{#each classes as c}<option value={c.id}>{c.course_code}</option>{/each}</select>
    <select bind:value={taskType}>{#each TASK_TYPES as type}<option value={type.value}>{type.label}</option>{/each}</select>
    <input type="number" min="0" placeholder="Estimate (min)" bind:value={estimate} />
    <input type="number" min="0" placeholder="Budget (min)" bind:value={budget} />
    <input placeholder="Tags · reading, brief" bind:value={tags} />
    <button type="submit" disabled={busy || !name.trim()}>Add template</button>
  </form>
  {#if templates.length}
    <ul class="template-list">{#each templates as template (template.id)}<li><button class="template-name" onclick={() => void selectTemplate(template.id)}><strong>{template.name}</strong><small>{template.default_estimated_minutes ? `${template.default_estimated_minutes}m` : "No estimate"} · {template.task_type ?? "assignment"}</small></button><button class="quiet" disabled={busy} onclick={() => { busy = true; void deleteTaskTemplate(template.id).then(refresh).catch(() => error = "Couldn't delete this template.").finally(() => busy = false); }}>Delete</button></li>{/each}</ul>
  {:else}<p class="empty">No templates yet.</p>{/if}
  {#if selectedTemplate !== null}<div class="milestone-editor"><h3>Milestone preset</h3><p class="hint">Offsets are counted back from the assignment due date.</p>{#each milestones as milestone (milestone.id)}<div class="milestone-row"><span>{milestone.title}<small>{milestone.offset_days_before_due === null ? "No due-date offset" : `${milestone.offset_days_before_due} days before due`}</small></span><button class="quiet" onclick={() => void deleteTemplateMilestone(milestone.id).then(() => selectTemplate(selectedTemplate!))}>Delete</button></div>{/each}<form class="milestone-form" onsubmit={(e) => { e.preventDefault(); void addMilestone(); }}><input required placeholder="Research" bind:value={milestoneTitle} /><input type="number" min="0" placeholder="Days before due" bind:value={milestoneOffset} /><button type="submit" disabled={busy}>Add milestone</button></form></div>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .hint { color: var(--muted); font-size: 12px; }.template-form { display: grid; grid-template-columns: 2fr 1fr 1fr; gap: 8px; }.template-form input:first-child, .template-form input:last-of-type { grid-column: span 2; }.template-form button { grid-column: 1 / -1; justify-self: start; }.template-list { list-style: none; padding: 0; display: grid; gap: 6px; }.template-list li { display: flex; justify-content: space-between; align-items: center; padding: 5px 10px; border: 1px solid var(--line); border-radius: 6px; }.template-name { display:grid; gap:3px; text-align:left; border:0; background:transparent; padding:5px 0; }.template-list small,.milestone-row small { color: var(--muted); font-size: 10px; display:block; }.quiet { border-color: transparent; background: transparent; }.empty { padding: 10px; }.error { color: var(--danger); }.milestone-editor { margin-top:12px; padding:12px; border-top:1px solid var(--line); }.milestone-row { display:flex; justify-content:space-between; align-items:center; padding:6px 0; border-bottom:1px solid var(--line); }.milestone-form { display:grid; grid-template-columns:1fr 1fr auto; gap:6px; margin-top:9px; }@media(max-width:650px){.template-form{grid-template-columns:1fr}.template-form input:first-child,.template-form input:last-of-type{grid-column:auto}.milestone-form{grid-template-columns:1fr}}
</style>
