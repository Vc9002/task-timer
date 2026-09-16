<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Modal from "./Modal.svelte";
  import { timerStore } from "$lib/stores/timer.svelte";
  let { onclose }: { onclose: () => void } = $props();
  type Task = { id: number; title: string; course_code: string; class_name: string | null };
  let tasks = $state<Task[]>([]);
  let query = $state("");
  let selected = $state(0);
  let error = $state("");
  let loading = $state(true);
  let resultsElement: HTMLDivElement;
  let results = $derived(tasks.filter(t => `${t.title} ${t.course_code} ${t.class_name ?? ""}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())));
  $effect(() => { const index = selected; resultsElement?.querySelector(`[data-index="${index}"]`)?.scrollIntoView({ block: "nearest" }); });
  onMount(() => { void invoke<Task[]>("list_startable_tasks").then(value => tasks = value).catch(() => error = "Couldn't load tasks. Close and try again.").finally(() => loading = false); });
  async function start(task: Task) {
    await timerStore.start(task.id, `${task.course_code} — ${task.title}`);
    if (timerStore.conflict || !timerStore.error) onclose();
  }
</script>
<Modal title="Start Task" {onclose}>
  <label>Search tasks and classes
    <input value={query} oninput={e => { query = e.currentTarget.value; selected = 0; }} onkeydown={e => {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); selected = Math.max(0, Math.min(results.length - 1, selected + (e.key === "ArrowDown" ? 1 : -1))); }
      if (e.key === "Enter" && results[selected] && !timerStore.busy) { e.preventDefault(); void start(results[selected]); }
    }} />
  </label>
  <p class="hint">↑ ↓ choose · Enter start · Esc close</p>
  <div class="results" bind:this={resultsElement}>
    {#each results as task, i (task.id)}
      <button data-index={i} class:selected={i === selected} aria-pressed={i === selected} disabled={timerStore.busy} onclick={() => start(task)}><small>{task.course_code}</small> {task.title}</button>
    {:else}<p>{loading ? "Loading…" : "No incomplete tasks found."}</p>{/each}
  </div>
  {#if error || timerStore.error}<p role="alert">{error || timerStore.error}</p>{/if}
</Modal>
<style>
input { display: block; width: 100%; margin-top: 6px; }
  label { font-size: 12px; color: var(--muted); }
  .hint { font-size: 10px; color: var(--muted); margin: 10px 0 15px; }
  .results { max-height: 45vh; overflow: auto; }
  button { display: block; width: 100%; text-align: left; padding: 12px; border: 1px solid transparent; background: transparent; color: var(--text); margin-bottom: 4px; }
  button.selected { background: var(--accent-soft); border-color: var(--accent); }
  small { display: block; font-size: 10px; margin-bottom: 4px; color: var(--muted); font-weight: 500; }
</style>
