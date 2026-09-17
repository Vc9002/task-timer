<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import Modal from "./Modal.svelte";
  import { listClasses, listTasksForClass, type ClassRecord, type TaskRecord } from "$lib/api";
  import { timerStore } from "$lib/stores/timer.svelte";

  let { onclose, onquickadd, onstart }: {
    onclose: () => void;
    onquickadd: () => void;
    onstart: () => void;
  } = $props();

  type Result =
    | { kind: "navigation"; label: string; detail: string; href: string }
    | { kind: "action"; label: string; detail: string; run: () => void }
    | { kind: "task"; label: string; detail: string; task: TaskRecord; classCode: string };

  const navigation: Result[] = [
    { kind: "navigation", label: "Today", detail: "Due and planned work", href: "/" },
    { kind: "navigation", label: "Inbox", detail: "Unscheduled work", href: "/inbox" },
    { kind: "navigation", label: "Week", detail: "Plan the next seven days", href: "/week" },
    { kind: "navigation", label: "Calendar", detail: "Monthly deadlines and study dates", href: "/calendar" },
    { kind: "navigation", label: "Classes", detail: "Courses and local tasks", href: "/classes" },
    { kind: "navigation", label: "History", detail: "Review tracked sessions", href: "/history" },
    { kind: "navigation", label: "Analytics", detail: "See where time went", href: "/analytics" },
    { kind: "navigation", label: "Settings", detail: "Integrations and preferences", href: "/settings" },
  ];
  let classes = $state<ClassRecord[]>([]);
  let tasks = $state<Result[]>([]);
  let query = $state("");
  let selected = $state(0);
  let loading = $state(true);
  let error = $state("");
  let resultsElement: HTMLDivElement;

  let actions: Result[] = $derived([
    { kind: "action", label: "Add a task", detail: "Create a local task", run: onquickadd },
    { kind: "action", label: timerStore.active ? "Switch task" : "Start a task", detail: "Begin tracking time", run: onstart },
  ]);
  let classResults: Result[] = $derived(classes.map(c => ({
    kind: "navigation" as const,
    label: c.course_code,
    detail: c.name ? `${c.name} · Class` : "Class",
    href: "/classes",
  })));
  let allResults: Result[] = $derived([...actions, ...navigation, ...classResults, ...tasks]);
  let results = $derived(allResults.filter(result =>
    `${result.label} ${result.detail}`.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase())
  ));

  $effect(() => {
    const index = selected;
    resultsElement?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" });
  });

  onMount(() => {
    void listClasses().then(async value => {
      classes = value;
      const grouped = await Promise.all(value.map(async c => ({ classCode: c.course_code, tasks: await listTasksForClass(c.id) })));
      tasks = grouped.flatMap(group => group.tasks
        .filter(task => task.status !== "completed" && task.external_state === "active")
        .map(task => ({ kind: "task" as const, label: task.title, detail: `${group.classCode} · ${task.source === "todoist" ? "Todoist" : "Local"}`, task, classCode: group.classCode })));
    }).catch(() => error = "Couldn't load local tasks. Navigation still works.")
      .finally(() => loading = false);
  });

  function closeAfterAction() {
    onclose();
  }

  async function choose(result: Result) {
    if (result.kind === "navigation") {
      closeAfterAction();
      await goto(result.href);
    } else if (result.kind === "action") {
      closeAfterAction();
      result.run();
    } else {
      await timerStore.start(result.task.id, `${result.classCode} — ${result.task.title}`);
      if (!timerStore.conflict && !timerStore.error) closeAfterAction();
    }
  }

  function move(delta: number) {
    if (results.length === 0) return;
    selected = (selected + delta + results.length) % results.length;
  }
</script>

<Modal title="Command palette" {onclose}>
  <label class="search-label">Search tasks, classes, or views
    <input placeholder="Try “week” or a task title" value={query} oninput={event => { query = event.currentTarget.value; selected = 0; }} onkeydown={event => {
      if (event.key === "ArrowDown") { event.preventDefault(); move(1); }
      if (event.key === "ArrowUp") { event.preventDefault(); move(-1); }
      if (event.key === "Enter" && results[selected] && !timerStore.busy) { event.preventDefault(); void choose(results[selected]); }
    }} />
  </label>
  <p class="hint"><kbd>↑</kbd> <kbd>↓</kbd> choose · <kbd>Enter</kbd> open · <kbd>Esc</kbd> close</p>
  <div class="results" bind:this={resultsElement}>
    {#each results as result, i (result.kind === "task" ? `task-${result.task.id}` : `${result.kind}-${result.label}`)}
      <button type="button" data-index={i} class:selected={i === selected} aria-pressed={i === selected} disabled={timerStore.busy} onclick={() => void choose(result)}>
        <span class="result-main">{result.label}</span><small>{result.detail}</small>
      </button>
    {:else}
      <p class="empty">{loading ? "Loading local work…" : "No matching commands or tasks."}</p>
    {/each}
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</Modal>

<style>
  .search-label { display: block; color: var(--muted); font-size: 12px; }
  input { display: block; width: 100%; margin-top: 6px; font-size: 15px; }
  .hint { margin: 10px 0 15px; color: var(--muted); font-size: 10px; }
  .hint kbd { margin-right: 2px; }
  .results { max-height: 52vh; overflow: auto; margin: 0 -8px; }
  .results button { width: 100%; justify-content: space-between; align-items: baseline; gap: 16px; padding: 11px 12px; border-color: transparent; background: transparent; text-align: left; }
  .results button.selected { background: var(--accent-soft); border-color: var(--accent); }
  .result-main { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  small { flex-shrink: 0; color: var(--muted); font-size: 10px; }
  .empty { padding: 16px 12px; color: var(--muted); font-size: 12px; }
  .error { margin-bottom: 0; }
</style>
