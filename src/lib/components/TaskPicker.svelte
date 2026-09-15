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
  let results = $derived(tasks.filter(t => `${t.title} ${t.course_code} ${t.class_name ?? ""}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())));
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
  <div class="results">
    {#each results as task, i (task.id)}
      <button class:selected={i === selected} disabled={timerStore.busy} onclick={() => start(task)}><small>{task.course_code}</small> {task.title}</button>
    {:else}<p>{loading ? "Loading…" : "No incomplete tasks found."}</p>{/each}
  </div>
  {#if error || timerStore.error}<p role="alert">{error || timerStore.error}</p>{/if}
</Modal>
<style>
  input { display: block; width: 100%; box-sizing: border-box; padding: .6rem; }
  .hint { font-size: .8rem; color: #777; }
  .results { max-height: 45vh; overflow: auto; }
  button { display: block; width: 100%; text-align: left; padding: .7rem; border: 1px solid transparent; background: transparent; color: inherit; }
  button.selected { border-color: #396cd8; }
  small { display: block; color: #777; }
</style>
