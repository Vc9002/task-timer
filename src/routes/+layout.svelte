<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/stores";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { pomodoroStore } from "$lib/stores/pomodoro.svelte";
  import { updaterStore } from "$lib/stores/updater.svelte";
  import { themeStore } from "$lib/stores/theme.svelte";
  import { toastStore } from "$lib/stores/toast.svelte";
  import Modal from "$lib/components/Modal.svelte";
  import { formatHms } from "$lib/format";
  import QuickAdd from "$lib/components/QuickAdd.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import TaskPicker from "$lib/components/TaskPicker.svelte";
  import CommandPalette from "$lib/components/CommandPalette.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import "$lib/app.css";
  let pickerOpen = $state(false);
  let commandPaletteOpen = $state(false);
  let focusMode = $state(false);
  const shortcutHint = typeof navigator !== "undefined" && /mac/i.test(navigator.platform) ? "⌘K" : "Ctrl K";

  let { children } = $props();
  let isQuickCapture = $derived($page.url.pathname === "/quick-capture");
  let isMiniTimer = $derived($page.url.pathname === "/mini-timer");
  let isBareWindow = $derived(isQuickCapture || isMiniTimer);

  onMount(() => {
    if (isQuickCapture) {
      themeStore.load();
      return;
    }
    if (isMiniTimer) {
      themeStore.load();
      void timerStore.mount();
      let disposed = false;
      const unlisteners: UnlistenFn[] = [];
      const addListener = (promise: Promise<UnlistenFn>) => void promise.then(unlisten => { if (disposed) unlisten(); else unlisteners.push(unlisten); }).catch(() => {});
      addListener(listen("timer-changed", () => timerStore.externalChange()));
      const refresh = () => { if (document.visibilityState === "visible") void timerStore.refresh(); };
      window.addEventListener("focus", refresh);
      document.addEventListener("visibilitychange", refresh);
      return () => {
        disposed = true;
        unlisteners.forEach(unlisten => unlisten());
        timerStore.destroy();
        window.removeEventListener("focus", refresh);
        document.removeEventListener("visibilitychange", refresh);
      };
    }
    const onKeydown = (event: KeyboardEvent) => {
      if (!event.repeat && (event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") { event.preventDefault(); openCommandPalette(); }
    };
    window.addEventListener("keydown", onKeydown);
    void timerStore.mount().then(() => pomodoroStore.loadSettings());
    void updaterStore.checkInBackground();
    themeStore.load();
    focusMode = localStorage.getItem("tasktimer-focus-mode") === "true";
    void invoke("set_focus_mode", { active: focusMode });
    let disposed = false;
    const unlisteners: UnlistenFn[] = [];
    const addListener = (promise: Promise<UnlistenFn>) => void promise.then(unlisten => { if (disposed) unlisten(); else unlisteners.push(unlisten); }).catch(() => timerStore.error = "Desktop controls couldn't connect. Reopen TaskTimer.");
    const action = async () => {
      const pending = await invoke<string | null>("take_desktop_action");
      if (disposed) return;
      if (pending === "start-task") pickerOpen = true;
      if (pending === "quick-add") await openQuickAdd();
    };
    addListener(listen("timer-changed", () => timerStore.externalChange()));
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
      window.removeEventListener("keydown", onKeydown);
      window.removeEventListener("focus", refresh);
      window.removeEventListener("pageshow", refresh);
      document.removeEventListener("visibilitychange", refresh);
    };
  });

  let editingRecovery = $state(false);
  let recoveryMinutes = $state<number | undefined>(0);
  let quickAddOpen = $state(false);

  function toggleFocusMode() {
    focusMode = !focusMode;
    localStorage.setItem("tasktimer-focus-mode", String(focusMode));
    void invoke("set_focus_mode", { active: focusMode });
  }

  const links = [
    { href: "/", label: "Today", icon: "today" },
    { href: "/inbox", label: "Inbox", icon: "inbox" },
    { href: "/week", label: "Week", icon: "week" },
    { href: "/calendar", label: "Calendar", icon: "calendar" },
    { href: "/classes", label: "Classes", icon: "classes" },
    { href: "/history", label: "History", icon: "history" },
    { href: "/analytics", label: "Analytics", icon: "analytics" },
    { href: "/semester", label: "Semester", icon: "semester" },
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

  function openCommandPalette() {
    commandPaletteOpen = true;
  }
</script>

{#if isBareWindow}
  {@render children()}
{:else}
{#if pickerOpen}<TaskPicker onclose={() => pickerOpen = false} />{/if}
{#if commandPaletteOpen}
  <CommandPalette onclose={() => commandPaletteOpen = false} onquickadd={openQuickAdd} onstart={() => pickerOpen = true} />
{/if}

{#if quickAddOpen}
  <QuickAdd onclose={() => quickAddOpen = false} />
{/if}

{#if updaterStore.ready}
  <div class="update-toast" role="status">
    <span>An update has been downloaded.</span>
    <button class="primary" disabled={updaterStore.installing} onclick={() => updaterStore.restartNow()}>
      {updaterStore.installing ? "Restarting…" : "Restart now"}
    </button>
    <button class="quiet" onclick={() => updaterStore.dismiss()}>Later</button>
  </div>
{/if}

{#if toastStore.current}
  <div class="toast" role="status">
    <span>{toastStore.current.message}</span>
    {#if toastStore.current.actionLabel}<button onclick={() => toastStore.runAction()}>{toastStore.current.actionLabel}</button>{/if}
    <button class="quiet toast-close" aria-label="Dismiss" onclick={() => toastStore.dismiss()}><Icon name="close" size={13} /></button>
  </div>
{/if}

{#if timerStore.conflict}
  <Modal title="Switch timer?" onclose={() => { if (!timerStore.busy) timerStore.conflict = null; }}>
    <dl class="facts">
      <dt>Tracking now</dt><dd>{timerStore.conflict.classCode} — {timerStore.conflict.taskTitle} <span class="tabular muted">· {formatHms(timerStore.displaySeconds)}</span></dd>
      <dt>Switch to</dt><dd>{timerStore.conflict.requestedTitle}</dd>
    </dl>
    <div class="modal-actions">
      <button class="primary" disabled={timerStore.busy} onclick={() => timerStore.switchToRequested()}>Finish current &amp; start new</button>
      <button disabled={timerStore.busy} onclick={() => timerStore.conflict = null}>Keep current</button>
    </div>
    {#if timerStore.error}<p role="alert">{timerStore.error}</p>{/if}
  </Modal>
{:else if timerStore.recovery && timerStore.active}
  <Modal title="Unfinished timer found" onclose={() => timerStore.recovery = false}>
    <dl class="facts">
      <dt>Task</dt><dd>{timerStore.active.class_course_code} — {timerStore.active.task_title}</dd>
      <dt>Started</dt><dd>{new Date(timerStore.active.session.start_ts.replace(" ", "T") + "Z").toLocaleString(undefined, { weekday: "short", month: "short", day: "numeric", hour: "numeric", minute: "2-digit" })}</dd>
      <dt>Elapsed</dt><dd class="tabular">{formatHms(timerStore.displaySeconds)}{timerStore.active.is_paused ? " (paused)" : ""}</dd>
    </dl>
    {#if editingRecovery}
      <label class="recovery-edit">Tracked minutes <input type="number" min="0" step="0.1" bind:value={recoveryMinutes} /></label>
      <div class="modal-actions">
        <button class="primary" disabled={timerStore.busy || recoveryMinutes === undefined || recoveryMinutes < 0} onclick={() => timerStore.finishEdited(recoveryMinutes ?? 0)}>Save &amp; finish</button>
        <button disabled={timerStore.busy} onclick={() => editingRecovery = false}>Back</button>
      </div>
    {:else}
      <div class="modal-actions">
        <button class="primary" onclick={() => timerStore.recovery = false}>Keep tracking</button>
        <button disabled={timerStore.busy} onclick={() => timerStore.finish()}>Finish now</button>
        <button onclick={() => { recoveryMinutes = Math.round(timerStore.displaySeconds / 6) / 10; editingRecovery = true; }}>Edit time</button>
        <span class="spacer"></span>
        <button class="danger" disabled={timerStore.busy} onclick={() => timerStore.cancel()}>Discard</button>
      </div>
    {/if}
    {#if timerStore.error}<p role="alert">{timerStore.error}</p>{/if}
  </Modal>
{/if}
{#if focusMode}
  <button class="focus-exit" onclick={toggleFocusMode} title="Exit Focus Mode">
    <Icon name="close" size={13} /> Exit Focus
  </button>
{/if}
<div class="shell" class:focus-mode={focusMode}>
  <aside class="sidebar">
    <a href="/" class="brand"><span class="brand-icon"><Icon name="clock" size={21} /></span>TaskTimer</a>
    <button class="quick-add" onclick={openCommandPalette}><Icon name="search" size={16} /> Search <kbd>{shortcutHint}</kbd></button>
    <button class="focus-toggle" onclick={toggleFocusMode}><Icon name="focus" size={14} />{focusMode ? "Exit Focus Mode" : "Focus Mode"}</button>
    <nav aria-label="Main navigation">
    {#each links as link}
      <a href={link.href} class:active={$page.url.pathname === link.href} aria-current={$page.url.pathname === link.href ? "page" : undefined}><Icon name={link.icon} />{link.label}</a>
    {/each}
    </nav>
    <div class="pomodoro-widget" class:on-break={pomodoroStore.phase === "break"}>
      {#if pomodoroStore.phase === "idle"}
        <button class="pomodoro-start" onclick={() => pomodoroStore.start()}><Icon name="clock" size={14} />Start Pomodoro</button>
      {:else}
        <span class="pomodoro-phase">{pomodoroStore.phase === "work" ? "Focus" : pomodoroStore.onLongBreak ? "Long break" : "Break"}</span>
        <span class="pomodoro-clock">{formatHms(pomodoroStore.remainingSeconds)}</span>
        <div class="pomodoro-actions">
          <button onclick={() => pomodoroStore.skip()}>Skip</button>
          <button onclick={() => pomodoroStore.stop()}>Stop</button>
        </div>
      {/if}
    </div>
    <div class="sidebar-bottom"><button class="primary" onclick={() => pickerOpen = true}><Icon name="play" size={15} />{timerStore.active ? "Switch task" : "Start a task"}</button></div>
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
      <button class="secondary-action" onclick={toggleFocusMode} title={focusMode ? "Exit Focus Mode" : "Focus Mode: hide the sidebar"}><Icon name="focus" size={15} /><span>{focusMode ? "Exit Focus" : "Focus"}</span></button>
      <button class="secondary-action" onclick={() => void invoke("toggle_mini_timer").catch(() => {})} title="Show or hide the floating mini timer"><Icon name="mini" size={15} /><span>Mini</span></button>
      <button disabled={timerStore.busy} onclick={togglePause}><Icon name={timerStore.active.is_paused ? "play" : "pause"} size={15} />{timerStore.active.is_paused ? "Resume" : "Pause"}</button>
      <button disabled={timerStore.busy} onclick={finish} class="primary"><Icon name="check" size={16} />Finish</button>
    </div>
  {/if}
  </div>
</div>
{/if}

<style>
.shell { display: flex; height: 100dvh; overflow: hidden; }
  .sidebar { width: 184px; flex-shrink: 0; padding: 22px 14px 14px; display: flex; flex-direction: column; background: var(--sidebar); border-right: 1px solid var(--line); }
  .brand { display: flex; align-items: center; gap: 9px; padding: 0 8px 20px; flex-shrink: 0; font-size: 16px; letter-spacing: -.5px; font-weight: 700; text-decoration: none; color: var(--text); }
  .brand-icon { display: flex; color: var(--accent); }
  .quick-add { justify-content: flex-start; padding: 8px; font-size: 12px; margin-bottom: 6px; color: var(--muted); flex-shrink: 0; }
  .focus-toggle { justify-content: flex-start; padding: 7px 8px; font-size: 12px; margin-bottom: 14px; flex-shrink: 0; color: var(--muted); border-color: transparent; background: transparent; }
  .focus-toggle:hover { color: var(--text); background: var(--hover); }
  .quick-add kbd { margin-left: auto; white-space: nowrap; }
  nav { display: grid; gap: 3px; align-content: start; flex: 1; min-height: 0; overflow-y: auto; margin-inline: -4px; padding-inline: 4px; }
  nav a { display: flex; align-items: center; gap: 10px; padding: 8px 12px; transition: background-color .12s ease-out, color .12s ease-out; color: var(--muted); text-decoration: none; border-radius: 6px; font-size: 13px; font-weight: 500; }
  nav a:hover { background: var(--hover); color: var(--text); }
  nav a.active { color: var(--accent); background: var(--accent-soft); font-weight: 650; }
  .pomodoro-widget { margin-top: 12px; flex-shrink: 0; padding: 10px; border: 1px solid var(--line); border-radius: 8px; display: grid; gap: 6px; font-size: 12px; }
  .pomodoro-widget.on-break { border-color: var(--accent); }
  .pomodoro-widget:has(.pomodoro-start) { padding: 0; border: 0; }
  .pomodoro-start { justify-content: flex-start; padding: 7px 8px; font-size: 12px; color: var(--muted); }
  .pomodoro-phase { color: var(--muted); font-size: 10px; text-transform: uppercase; letter-spacing: .06em; }
  .pomodoro-clock { font-variant-numeric: tabular-nums; font-size: 18px; font-weight: 600; letter-spacing: -.02em; }
  .pomodoro-actions { display: flex; gap: 6px; }
  .pomodoro-actions button { flex: 1; padding: 4px; font-size: 11px; }
  .sidebar-bottom { display: grid; padding-top: 10px; flex-shrink: 0; }
  .workspace { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .focus-mode .sidebar { display: none; }
  .focus-mode .content { max-width: 1080px; width: 100%; margin-inline: auto; }
  .content { overflow-y: auto; flex: 1; }
  .app-error { padding: 8px 20px; border-bottom: 1px solid var(--line); }
  .update-toast { position: fixed; right: 20px; bottom: 20px; z-index: 50; display: flex; align-items: center; gap: 10px; padding: 12px 14px; border: 1px solid var(--line); border-radius: 10px; background: var(--surface); box-shadow: var(--shadow); font-size: 13px; }
  .focus-exit { position: fixed; top: 14px; right: 14px; z-index: 50; display: flex; align-items: center; gap: 6px; padding: 6px 12px; font-size: 12px; border-radius: 20px; background: var(--surface); border: 1px solid var(--line); box-shadow: var(--shadow); color: var(--muted); }
  .focus-exit:hover { color: var(--text); background: var(--hover); }
  .timer-bar { display: flex; align-items: center; gap: 10px; flex-shrink: 0; padding: 12px 24px; border-top: 1px solid var(--line); background: var(--surface); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); flex-shrink: 0; }
  .dot.paused { background: var(--warn); }
  .label { flex: 1; min-width: 0; }
  .label small { display: block; color: var(--muted); font-size: 11px; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; }
  .label strong { display: block; white-space: nowrap; text-overflow: ellipsis; overflow: hidden; font-size: 13px; font-weight: 600; }
  .toast { position: fixed; left: 50%; bottom: 84px; transform: translateX(-50%); z-index: 60; display: flex; align-items: center; gap: 10px; max-width: min(460px, calc(100vw - 32px)); padding: 8px 8px 8px 14px; border: 1px solid var(--line); border-radius: 10px; background: var(--surface); box-shadow: var(--shadow); font-size: 13px; animation: toast-in .18s cubic-bezier(.16, 1, .3, 1); }
  .toast span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .toast button { min-height: 28px; padding: 2px 10px; font-size: 12px; flex-shrink: 0; }
  .toast .toast-close { width: 28px; padding: 0; color: var(--muted); }
  @keyframes toast-in { from { opacity: 0; transform: translate(-50%, 6px); } }
  .facts { display: grid; grid-template-columns: auto 1fr; gap: 8px 16px; margin: 0; font-size: 13px; }
  .facts dt { color: var(--muted); }
  .facts dd { margin: 0; min-width: 0; overflow-wrap: anywhere; }
  .muted { color: var(--muted); }
  .recovery-edit { display: grid; gap: 5px; margin-top: 16px; font-size: 12px; color: var(--muted); }
  .clock { font-variant-numeric: tabular-nums; font-size: 22px; font-weight: 600; letter-spacing: -.02em; margin-right: 6px; }
  @media (max-width: 700px) {
    .sidebar { width: 145px; padding-inline: 10px; }
    .quick-add kbd { display: none; }
    .brand { font-size: 14px; gap: 5px; }
    .timer-bar { padding: 10px 12px; gap: 8px; }
    .secondary-action span { display: none; }
    .secondary-action { width: 34px; padding: 0; }
    .clock { font-size: 18px; margin-right: 0; }
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
