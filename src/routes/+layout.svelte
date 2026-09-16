<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { timerStore } from "$lib/stores/timer.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { formatHms } from "$lib/format";
  import QuickAdd from "$lib/components/QuickAdd.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import TaskPicker from "$lib/components/TaskPicker.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import "$lib/app.css";
  let pickerOpen = $state(false);

  let { children } = $props();

  onMount(() => {
    timerStore.mount();
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const addListener = (promise: Promise<UnlistenFn>) => void promise.then(unlisten => { if (disposed) unlisten(); else unlisteners.push(unlisten); }).catch(() => timerStore.error = "Desktop controls couldn't connect. Reopen TaskTimer.");
    const action = async () => {
      const pending = await invoke<string | null>("take_desktop_action");
      if (disposed) return;
      if (pending === "start-task") pickerOpen = true;
      if (pending === "quick-add") await openQuickAdd();
    };
    addListener(listen("timer-changed", () => { void timerStore.refresh(); timerStore.revision++; }));
    addListener(listen<string>("desktop-error", event => timerStore.error = event.payload));
    addListener(listen("desktop-action", () => { void action(); }));
    void action().catch(() => {});
    const refresh = () => { if (document.visibilityState === "visible") void timerStore.refresh(); };
    window.addEventListener("focus", refresh);
    window.addEventListener("pageshow", refresh);
    document.addEventListener("visibilitychange", refresh);
    return () => {
      disposed = true;
      unlisteners.forEach(unlisten => unlisten());
      timerStore.destroy();
      window.removeEventListener("focus", refresh);
      window.removeEventListener("pageshow", refresh);
      document.removeEventListener("visibilitychange", refresh);
    };
  });

  let editingRecovery = $state(false);
  let recoveryMinutes = $state<number | undefined>(0);
  let quickAddOpen = $state(false);

  const links = [
    { href: "/", label: "Today", icon: "today" },
    { href: "/week", label: "Week", icon: "week" },
    { href: "/calendar", label: "Calendar", icon: "today" },
    { href: "/classes", label: "Classes", icon: "classes" },
    { href: "/history", label: "History", icon: "history" },
    { href: "/analytics", label: "Analytics", icon: "analytics" },
    { href: "/settings", label: "Settings", icon: "settings" },
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
    quickAddOpen = true;
  }
</script>

{#if pickerOpen}<TaskPicker onclose={() => pickerOpen = false} />{/if}

<svelte:window onkeydown={(event) => {
  if (!event.repeat && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); void openQuickAdd(); }
}} />

{#if quickAddOpen}
  <QuickAdd onclose={() => quickAddOpen = false} />
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
  <aside class="sidebar">
    <a href="/" class="brand"><span class="brand-icon"><Icon name="clock" size={21} /></span>TaskTimer</a>
    <button class="quick-add" onclick={openQuickAdd}><Icon name="plus" size={16} /> Add task <kbd>⌘ / Ctrl K</kbd></button>
    <nav aria-label="Main navigation">
    {#each links as link}
      <a href={link.href} class:active={$page.url.pathname === link.href} aria-current={$page.url.pathname === link.href ? "page" : undefined}><Icon name={link.icon} />{link.label}</a>
    {/each}
    </nav>
    <div class="sidebar-bottom"><span class="eyebrow">One task at a time</span><button class="primary" onclick={() => pickerOpen = true}><Icon name="play" size={15} />{timerStore.active ? "Switch task" : "Start a task"}</button></div>
  </aside>
  <div class="workspace">
    {#if timerStore.error}<div class="app-error" role="alert">{timerStore.error} <button onclick={() => timerStore.refresh()}>Refresh timer</button></div>{/if}
    <div class="content">
    {@render children()}
    </div>

  {#if timerStore.active}
    <div class="timer-bar">
      <span class="dot" class:paused={timerStore.active.is_paused}></span>
      <span class="label"><small>{timerStore.active.is_paused ? "Paused" : "Now tracking"} · {timerStore.active.class_course_code}</small><strong>{timerStore.active.task_title}</strong></span>
      <span class="clock">{formatHms(timerStore.displaySeconds)}</span>
      <button disabled={timerStore.busy} onclick={togglePause}><Icon name={timerStore.active.is_paused ? "play" : "pause"} size={15} />{timerStore.active.is_paused ? "Resume" : "Pause"}</button>
      <button disabled={timerStore.busy} onclick={finish} class="primary"><Icon name="check" size={16} />Finish</button>
    </div>
  {/if}
  </div>
</div>

<style>
.shell { display: flex; height: 100dvh; overflow: hidden; }
  .sidebar { width: 184px; flex-shrink: 0; padding: 27px 14px 18px; display: flex; flex-direction: column; background: var(--sidebar); border-right: 1px solid var(--line); }
  .brand { display: flex; align-items: center; gap: 9px; padding: 0 8px 25px; font-size: 16px; letter-spacing: -.5px; font-weight: 700; text-decoration: none; color: var(--text); }
  .brand-icon { display: flex; color: var(--accent); }
  .quick-add { justify-content: flex-start; padding: 8px; font-size: 12px; margin-bottom: 25px; box-shadow: 0 1px 2px #122c4307; }
  .quick-add kbd { font-size: 9px; margin-left: auto; padding: 1px 2px; border: 0; }
  nav { display: grid; gap: 5px; }
  nav a { display: flex; align-items: center; gap: 10px; padding: 9px 12px; color: var(--muted); text-decoration: none; border-radius: 6px; font-size: 13px; font-weight: 500; }
  nav a:hover { background: var(--hover); color: var(--text); }
  nav a.active { color: var(--accent); background: var(--accent-soft); font-weight: 650; }
  .sidebar-bottom { margin-top: auto; display: grid; gap: 10px; padding-top: 30px; }
  .sidebar-bottom .eyebrow { text-align: center; font-size: 9px; }
  .workspace { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .content { overflow-y: auto; flex: 1; }
  .app-error { padding: 8px 20px; border-bottom: 1px solid var(--line); }
  .timer-bar { display: flex; align-items: center; gap: 12px; flex-shrink: 0; padding: 16px 24px; border-top: 1px solid var(--line); background: var(--surface); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); flex-shrink: 0; }
  .dot.paused { background: #c59442; }
  .label { flex: 1; min-width: 0; }
  .label small { display: block; color: var(--muted); font-size: 10px; }
  .label strong { display: block; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; font-size: 12px; font-weight: 600; }
  .clock { font-family: ui-monospace, "SFMono-Regular", Consolas, monospace; font-variant-numeric: tabular-nums; font-size: 22px; letter-spacing: -1px; margin-right: 6px; }
  @media (max-width: 700px) {
    .sidebar { width: 145px; padding-inline: 10px; }
    .quick-add kbd { display: none; }
    .brand { font-size: 14px; gap: 5px; }
    .timer-bar { flex-wrap: wrap; padding: 12px; gap: 8px; }
    .label { min-width: 90px; }
    .clock { font-size: 18px; }
  }
  @media (max-width: 480px) {
    .shell { flex-direction: column; }
    .sidebar { width: 100%; padding: 10px; border-right: 0; border-bottom: 1px solid var(--line); flex-direction: row; flex-wrap: wrap; gap: 8px; }
    .brand { padding: 0; margin-right: auto; }
    .quick-add { margin: 0; }
    nav { width: 100%; display: flex; justify-content: space-between; }
    nav a { font-size: 11px; padding: 6px; gap: 4px; }
    nav a :global(svg) { display: none; }
    .sidebar-bottom { display: none; }
    .workspace { min-height: 0; }
  }
</style>
