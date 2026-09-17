<script lang="ts">
  import TaskTreeNode from "$lib/components/TaskTreeNode.svelte";
  import { listInbox, type InboxTask } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";

  let tasks = $state<InboxTask[]>([]);
  let error = $state("");
  let activeTag = $state<string | null>(null);
  let allTags = $derived(Array.from(new Set(tasks.flatMap(task => task.tags))).sort());
  let filtered = $derived(activeTag !== null ? tasks.filter(task => task.tags.includes(activeTag!)) : tasks);
  let groups = $derived(Array.from(new Set(filtered.map(task => `${task.course_code}\u0000${task.class_id}`))).map(key => {
    const [course_code, classId] = key.split("\u0000");
    return { course_code, class_id: Number(classId), tasks: filtered.filter(task => task.class_id === Number(classId)) };
  }));

  async function refresh() {
    try { tasks = await listInbox(); error = ""; }
    catch (e) { error = String(e); }
  }
  $effect(() => { void timerStore.revision; void refresh(); });
</script>

<main class="container">
  <header><div><p class="eyebrow">Unscheduled work</p><h1>Inbox</h1></div><span class="count">{tasks.length} {tasks.length === 1 ? "task" : "tasks"}</span></header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if allTags.length}
    <div class="tag-filter">
      <button class:active={activeTag === null} onclick={() => activeTag = null}>All</button>
      {#each allTags as tag (tag)}
        <button class:active={activeTag === tag} onclick={() => activeTag = activeTag === tag ? null : tag}>#{tag}</button>
      {/each}
    </div>
  {/if}
  {#if groups.length}
    <p class="page-intro">Work that has no planned date. Schedule it from the task menu or use the calendar.</p>
    {#each groups as group (group.class_id)}
      <section class="class-group"><h2>{group.course_code}</h2><ul>
        {#each group.tasks.filter(task => !group.tasks.some(parent => parent.id === task.parent_task_id)) as task (task.id)}
          <TaskTreeNode task={task} tasks={group.tasks} courseCode={group.course_code} onchanged={refresh} />
        {/each}
      </ul></section>
    {/each}
  {:else if !error && activeTag !== null}
    <div class="empty"><strong>No matches.</strong><p>Nothing tagged #{activeTag} is in the inbox.</p></div>
  {:else if !error}
    <div class="empty"><strong>Inbox zero.</strong><p>Every active task has a planned date.</p><a href="/classes">Add or review coursework →</a></div>
  {/if}
</main>

<style>
  header{display:flex;justify-content:space-between;align-items:end;gap:15px;margin-bottom:24px}.count{color:var(--muted);font-size:12px}.class-group{margin:28px 0}.class-group h2{font-size:11px;text-transform:uppercase;letter-spacing:.08em;color:var(--muted);margin:0 0 8px}.class-group h2::before{content:"";display:inline-block;width:5px;height:14px;border-radius:2px;background:var(--accent);margin-right:10px;vertical-align:-2px}.class-group ul{list-style:none;padding:0;margin:0}.empty strong{display:block;color:var(--text);font-weight:600}.empty p{font-size:12px}
  .tag-filter{display:flex;flex-wrap:wrap;gap:6px;margin-bottom:18px}.tag-filter button{font-size:11px;padding:4px 10px;border-radius:999px;border:1px solid var(--line);background:transparent;color:var(--muted)}.tag-filter button.active{background:var(--accent-soft);color:var(--accent);border-color:transparent}
</style>
