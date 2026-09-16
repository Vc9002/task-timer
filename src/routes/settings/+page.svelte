<script lang="ts">
  import { onMount } from "svelte";
  import DesktopSettings from "$lib/components/DesktopSettings.svelte";
  import StudyCapacitySettings from "$lib/components/StudyCapacitySettings.svelte";
  import {
    getTodoistStatus,
    setTodoistToken,
    disconnectTodoist,
    listTodoistProjectMappings,
    mapTodoistProject,
    syncTodoistNow,
    listClasses,
    type TodoistStatus,
    type TodoistProjectMapping,
    type ClassRecord,
    listRecurringTemplates,
    createRecurringTemplate,
    setRecurringTemplateActive,
    updateRecurringTemplate,
    type RecurringTemplate,
  } from "$lib/api";

  let status = $state<TodoistStatus | null>(null);
  let mappings = $state<TodoistProjectMapping[]>([]);
  let classes = $state<ClassRecord[]>([]);
  let token = $state("");
  let error = $state("");
  let busy = $state(false);
  let syncMessage = $state("");
  let recurring = $state<RecurringTemplate[]>([]);
  let recurringTitle = $state("");
  let recurringClass = $state<number | null>(null);
  let recurringType = $state<RecurringTemplate["recurrence_type"]>("weekly");
  let recurringStart = $state(new Date().toISOString().slice(0, 10));
  let recurringEstimate = $state("");
  let editingRecurring = $state<number | null>(null);

  async function refresh() {
    try {
      status = await getTodoistStatus();
      classes = await listClasses();
      recurring = await listRecurringTemplates();
      recurringClass ??= classes[0]?.id ?? null;
      if (status.connected) {
        mappings = await listTodoistProjectMappings();
      }
      error = "";
    } catch (e) {
      error = String(e);
    }
  }

  async function connect(event: Event) {
    event.preventDefault();
    if (!token.trim()) return;
    busy = true;
    try {
      await setTodoistToken(token.trim());
      token = "";
      await refresh();
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function disconnect() {
    try {
      await disconnectTodoist();
      mappings = [];
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function syncNow() {
    busy = true;
    syncMessage = "";
    try {
      const result = await syncTodoistNow();
      syncMessage = `Synced ${result.projects_synced} projects, ${result.tasks_synced} tasks.`;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function updateMapping(todoistId: string, classIdStr: string) {
    const classId = classIdStr ? Number(classIdStr) : null;
    try {
      await mapTodoistProject(todoistId, classId);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  onMount(refresh);

  async function addRecurring(event: Event) {
    event.preventDefault();
    if (!recurringClass || !recurringTitle.trim()) return;
    busy = true;
    try {
      await createRecurringTemplate({ class_id: recurringClass, title: recurringTitle.trim(), description: null, priority: 2, estimated_minutes: recurringEstimate ? Number(recurringEstimate) : null, recurrence_type: recurringType, interval: 1, weekdays: null, start_date: recurringStart, end_date: null });
      recurringTitle = ""; recurringEstimate = ""; recurring = await listRecurringTemplates();
    } catch (e) { error = String(e); } finally { busy = false; }
  }
  function editRecurring(template: RecurringTemplate) {
    editingRecurring = template.id;
    recurringTitle = template.title; recurringClass = template.class_id;
    recurringType = template.recurrence_type; recurringStart = template.start_date;
    recurringEstimate = template.estimated_minutes?.toString() ?? "";
  }
  async function saveRecurring(event: Event) {
    event.preventDefault();
    if (!editingRecurring || !recurringClass || !recurringTitle.trim()) return;
    busy = true;
    try {
      await updateRecurringTemplate({ id: editingRecurring, class_id: recurringClass, title: recurringTitle.trim(), description: null, priority: 2, estimated_minutes: recurringEstimate ? Number(recurringEstimate) : null, recurrence_type: recurringType, interval: 1, weekdays: null, start_date: recurringStart, end_date: null });
      editingRecurring = null; recurring = await listRecurringTemplates();
    } catch (e) { error = String(e); } finally { busy = false; }
  }
</script>

<main class="container">
  <h1>Settings</h1>
  <p class="page-intro">Make TaskTimer work the way you do.</p>
  <DesktopSettings />

  <section>
    <h2>Recurring tasks</h2>
    <p class="muted">Create lightweight daily or weekly study routines. Future occurrences are generated automatically; completed history stays intact.</p>
    <form class="recurring-form" onsubmit={editingRecurring ? saveRecurring : addRecurring}>
      <label>Task title<input required placeholder="Chinese vocabulary" bind:value={recurringTitle} /></label>
      <label>Class<select required bind:value={recurringClass}>{#each classes as c (c.id)}<option value={c.id}>{c.course_code}</option>{/each}</select></label>
      <label>Starts<input type="date" required bind:value={recurringStart} /></label>
      <label>Estimate (min)<input type="number" min="0" placeholder="30" bind:value={recurringEstimate} /></label>
      <button type="submit" disabled={busy || !recurringClass}>{editingRecurring ? "Save changes" : "Add recurring task"}</button>
      {#if editingRecurring}<button type="button" class="secondary" onclick={() => editingRecurring = null}>Cancel</button>{/if}
    </form>
    {#if recurring.length}<ul class="recurring-list">{#each recurring as template (template.id)}<li><span><strong>{template.title}</strong><small>{template.recurrence_type} · {template.start_date}</small></span><span class="row"><button class="secondary" onclick={() => editRecurring(template)}>Edit</button><button class="secondary" onclick={() => { void setRecurringTemplateActive(template.id, !template.active).then(() => listRecurringTemplates().then(v => recurring = v)); }}>{template.active ? "Stop" : "Stopped"}</button></span></li>{/each}</ul>{/if}
  </section>
  <StudyCapacitySettings />

  {#if error}<p class="error">{error}</p>{/if}

  <section>
    <h2>Todoist</h2>
    {#if status?.connected}
      <p class="status connected">Connected</p>
      {#if status.last_synced_at}
        <p class="muted">Last synced: {status.last_synced_at}</p>
      {/if}
      <div class="row">
        <button onclick={syncNow} disabled={busy}>Sync now</button>
        <button class="secondary" onclick={disconnect}>Disconnect</button>
      </div>
      {#if syncMessage}<p class="muted">{syncMessage}</p>{/if}

      <h3>Project mappings</h3>
      <p class="muted">Todoist course tags are matched automatically to active classes by exact course code (for example, <code>LGST 1000</code>). Project mappings remain available as an override.</p>
      <ul class="mappings">
        {#each mappings as m (m.todoist_id)}
          <li>
            <span class="name">{m.name}</span>
            <select aria-label={`Class for ${m.name}`} value={m.class_id ?? ""} onchange={(e) => updateMapping(m.todoist_id, (e.target as HTMLSelectElement).value)}>
              <option value="">Unmapped</option>
              {#each classes as c (c.id)}
                <option value={c.id}>{c.course_code}</option>
              {/each}
            </select>
          </li>
        {/each}
        {#if mappings.length === 0}
          <li class="empty">No projects yet — run Sync now.</li>
        {/if}
      </ul>
    {:else}
      <p class="status">Not connected</p>
      <form class="row" onsubmit={connect}>
        <input
          type="password"
          aria-label="Todoist API token"
          placeholder="Todoist API token"
          bind:value={token}
        />
        <button type="submit" disabled={busy}>Connect</button>
      </form>
      <p class="muted">
        Find your token in Todoist under Settings &rarr; Integrations &rarr; Developer.
        It's stored in your OS keychain, never in this app's database.
      </p>
    {/if}
  </section>
</main>

<style>
section { margin-bottom: 2rem; border-top: 1px solid var(--line); padding-top: 12px; }
  h2 { margin-top: 12px; }
  .status { font-size: 12px; font-weight: 600; }
  .status.connected { color: var(--accent); }
  .row { display: flex; gap: 8px; margin: 12px 0; }
  input, select { flex: 1; }
  .mappings { list-style: none; padding: 0; }
  .mappings li { display: flex; align-items: center; gap: 12px; padding: 10px 0; border-bottom: 1px solid var(--line); }
  .mappings .name { flex: 1; }
.muted { color: var(--muted); font-size: 12px; }
  .recurring-form { display:grid; grid-template-columns:2fr 1fr 1fr 1fr 1fr auto; gap:8px; align-items:end; margin:14px 0; }
  .recurring-form label { display:grid; gap:4px; font-size:11px; color:var(--muted); }
  .recurring-form button { white-space:nowrap; }
  .recurring-list { list-style:none; padding:0; margin:12px 0 0; }
  .recurring-list li { display:flex; justify-content:space-between; align-items:center; padding:9px 0; border-bottom:1px solid var(--line); }
  .recurring-list small { display:block; color:var(--muted); font-size:10px; }
  @media(max-width:800px) { .recurring-form { grid-template-columns:1fr 1fr; } .recurring-form label:first-child { grid-column:1/-1; } }
</style>
