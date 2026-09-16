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
  } from "$lib/api";

  let status = $state<TodoistStatus | null>(null);
  let mappings = $state<TodoistProjectMapping[]>([]);
  let classes = $state<ClassRecord[]>([]);
  let token = $state("");
  let error = $state("");
  let busy = $state(false);
  let syncMessage = $state("");

  async function refresh() {
    try {
      status = await getTodoistStatus();
      classes = await listClasses();
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
</script>

<main class="container">
  <h1>Settings</h1>
  <p class="page-intro">Make TaskTimer work the way you do.</p>
  <DesktopSettings />
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
</style>
