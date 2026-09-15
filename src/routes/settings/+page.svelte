<script lang="ts">
  import { onMount } from "svelte";
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
    if (classId === null) return;
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
            <select value={m.class_id ?? ""} onchange={(e) => updateMapping(m.todoist_id, (e.target as HTMLSelectElement).value)}>
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
  .container {
    max-width: 640px;
    margin: 0 auto;
    padding: 2.5rem 1.5rem;
  }

  section {
    margin-bottom: 2rem;
  }

  h2 {
    font-size: 1.1em;
    border-bottom: 1px solid #e5e5e5;
    padding-bottom: 0.4rem;
  }

  .status {
    font-weight: 600;
  }

  .status.connected {
    color: #2e7d32;
  }

  .row {
    display: flex;
    gap: 0.5rem;
    margin: 0.75rem 0;
  }

  input,
  select {
    flex: 1;
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

  button.secondary {
    background: #e5e5e5;
    color: #333;
  }

  .mappings {
    list-style: none;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .mappings li {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5em 0.7em;
    background: white;
    border-radius: 6px;
    border: 1px solid #ececec;
  }

  .mappings .name {
    flex: 1;
  }

  .muted {
    color: #888;
    font-size: 0.9em;
  }

  .empty {
    color: #888;
    font-style: italic;
  }

  .error {
    color: #b00020;
  }

  @media (prefers-color-scheme: dark) {
    .mappings li {
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
