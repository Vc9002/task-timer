<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { timerStore } from "$lib/stores/timer.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { formatHms } from "$lib/format";
  import { createTask, listClasses, type ClassRecord } from "$lib/api";

  let { children } = $props();

  onMount(() => {
    timerStore.mount();
    const refresh = () => { if (document.visibilityState === "visible") void timerStore.refresh(); };
    window.addEventListener("focus", refresh);
    window.addEventListener("pageshow", refresh);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      timerStore.destroy();
      window.removeEventListener("focus", refresh);
      window.removeEventListener("pageshow", refresh);
      document.removeEventListener("visibilitychange", refresh);
    };
  });

  let editingRecovery = $state(false);
  let recoveryMinutes = $state<number | undefined>(0);
  let quickAddOpen = $state(false);
  let quickTitle = $state("");
  let quickClass = $state<number | null>(null);
  let quickEstimate = $state("");
  let quickSchedule = $state("");
  let quickClasses = $state<ClassRecord[]>([]);
  let quickError = $state("");

  const links = [
    { href: "/", label: "Today" },
    { href: "/classes", label: "Classes" },
    { href: "/history", label: "History" },
    { href: "/analytics", label: "Analytics" },
    { href: "/settings", label: "Settings" },
  ];

  async function togglePause() {
    if (!timerStore.active) return;
    if (timerStore.active.is_paused) {
      await timerStore.resume();
    } else {
      await timerStore.pause();
    }
  }

  async function finish() {
    await timerStore.finish();
  }

  async function openQuickAdd() {
    quickClasses = await listClasses();
    quickClass = quickClasses[0]?.id ?? null;
    quickAddOpen = true;
    quickError = "";
  }
  async function addQuickTask() {
    if (!quickTitle.trim() || quickClass === null) return;
    try {
      await createTask({ class_id: quickClass, parent_task_id: null, title: quickTitle.trim(), description: null,
        priority: null, due_at: null, scheduled_date: quickSchedule || null,
        estimated_minutes: quickEstimate ? Number(quickEstimate) : null });
      quickTitle = ""; quickEstimate = ""; quickSchedule = ""; quickAddOpen = false;
    } catch { quickError = "Could not create this task. Try again."; }
  }
</script>

<svelte:window onkeydown={(event) => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); void openQuickAdd(); }
}} />

{#if quickAddOpen}
  <Modal title="Add Task" onclose={() => quickAddOpen = false}>
    <form onsubmit={(event) => { event.preventDefault(); void addQuickTask(); }}>
      <label>Task <input bind:value={quickTitle} /></label>
      <label>Class <select bind:value={quickClass}>{#each quickClasses as c}<option value={c.id}>{c.course_code}</option>{/each}</select></label>
      <label>Schedule <input type="date" bind:value={quickSchedule} /></label>
      <label>Estimate (minutes) <input type="number" min="0" bind:value={quickEstimate} /></label>
      <button type="submit" disabled={!quickTitle.trim() || quickClass === null}>Add</button>
      {#if quickError}<p role="alert">{quickError}</p>{/if}
    </form>
  </Modal>
{/if}

{#if timerStore.conflict}
  <Modal title="Switch timer?" onclose={() => { if (!timerStore.busy) timerStore.conflict = null; }}>
    <p>You're currently tracking {timerStore.conflict.classCode} — {timerStore.conflict.taskTitle}</p>
    <p>{formatHms(timerStore.displaySeconds)}</p>
    <p>You tried to start: {timerStore.conflict.requestedTitle}</p>
    <button disabled={timerStore.busy} onclick={() => timerStore.switchToRequested()}>Finish Current &amp; Start New</button>
    <button disabled={timerStore.busy} onclick={() => timerStore.conflict = null}>Keep Current</button>
    {#if timerStore.error}<p role="alert">{timerStore.error}</p>{/if}
  </Modal>
{:else if timerStore.recovery && timerStore.active}
  <Modal title="Unfinished timer found" onclose={() => timerStore.recovery = false}>
    <p>{timerStore.active.class_course_code} — {timerStore.active.task_title}</p>
    <p>Started: {new Date(timerStore.active.session.start_ts.replace(" ", "T") + "Z").toLocaleString()}</p>
    <p>Elapsed: {formatHms(timerStore.displaySeconds)} {timerStore.active.is_paused ? "(paused)" : ""}</p>
    {#if editingRecovery}
      <label>Tracked minutes <input type="number" min="0" step="0.1" bind:value={recoveryMinutes} /></label>
      <button disabled={timerStore.busy || recoveryMinutes === undefined || recoveryMinutes < 0} onclick={() => timerStore.finishEdited(recoveryMinutes ?? 0)}>Save &amp; Finish</button>
    {:else}
      <button onclick={() => timerStore.recovery = false}>Continue</button>
      <button disabled={timerStore.busy} onclick={() => timerStore.finish()}>Finish Now</button>
      <button onclick={() => { recoveryMinutes = Math.round(timerStore.displaySeconds / 6) / 10; editingRecovery = true; }}>Edit</button>
      <button disabled={timerStore.busy} onclick={() => timerStore.cancel()}>Discard</button>
    {/if}
    {#if timerStore.error}<p role="alert">{timerStore.error}</p>{/if}
  </Modal>
{/if}
<div class="shell">
  {#if timerStore.error}<p role="alert">{timerStore.error} <button onclick={() => timerStore.refresh()}>Refresh timer</button></p>{/if}
  <nav>
    {#each links as link}
      <a href={link.href} class:active={$page.url.pathname === link.href}>{link.label}</a>
    {/each}
  </nav>

  <div class="content">
    {@render children()}
  </div>

  {#if timerStore.active}
    <div class="timer-bar">
      <span class="dot" class:paused={timerStore.active.is_paused}></span>
      <span class="label">
        {timerStore.active.class_course_code} — {timerStore.active.task_title}
      </span>
      <span class="clock">{formatHms(timerStore.displaySeconds)}</span>
      <button disabled={timerStore.busy} onclick={togglePause}>{timerStore.active.is_paused ? "Resume" : "Pause"}</button>
      <button disabled={timerStore.busy} onclick={finish} class="finish">Finish</button>
    </div>
  {/if}
</div>

<style>
  :global(:root) {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    color: #0f0f0f;
    background-color: #f6f6f6;
  }

  :global(body) {
    margin: 0;
  }

  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  nav {
    display: flex;
    gap: 1.25rem;
    padding: 0.9rem 1.5rem;
    border-bottom: 1px solid #e0e0e0;
    background: white;
  }

  nav a {
    text-decoration: none;
    color: #555;
    font-weight: 500;
  }

  nav a.active {
    color: #396cd8;
  }

  .content {
    flex: 1;
    padding-bottom: 4.5rem;
  }

  .timer-bar {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.7rem 1.5rem;
    background: #1f2430;
    color: white;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #4caf50;
  }

  .dot.paused {
    background: #e0a300;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .clock {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    font-size: 1.1em;
  }

  .timer-bar button {
    padding: 0.4em 0.9em;
    border-radius: 6px;
    border: 1px solid transparent;
    background: #3a4155;
    color: white;
    cursor: pointer;
  }

  .timer-bar button.finish {
    background: #396cd8;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      color: #f6f6f6;
      background-color: #2f2f2f;
    }
    nav {
      background: #262626;
      border-color: #3a3a3a;
    }
  }
</style>
