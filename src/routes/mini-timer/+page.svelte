<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatHms } from "$lib/format";
  import Icon from "$lib/components/Icon.svelte";
</script>

<div class="mini" data-tauri-drag-region>
  <button
    type="button"
    class="close-btn"
    aria-label="Close mini timer"
    title="Close"
    onclick={() => void invoke("toggle_mini_timer")}
  >&times;</button>
  {#if timerStore.active}
    <span
      class="dot"
      class:paused={timerStore.active.is_paused}
      class:error={!!timerStore.error}
      title={timerStore.error || undefined}
      data-tauri-drag-region
    ></span>
    <div class="info" data-tauri-drag-region>
      <strong>{timerStore.active.task_title}</strong>
      <span class="clock">{formatHms(timerStore.displaySeconds)}</span>
    </div>
    <div class="actions">
      <button
        class="quiet"
        onclick={() => timerStore.active?.is_paused ? timerStore.resume() : timerStore.pause()}
        title={timerStore.active.is_paused ? "Resume" : "Pause"}
      ><Icon name={timerStore.active.is_paused ? "play" : "pause"} size={13} /></button>
      <button class="quiet" onclick={() => timerStore.finish()} title="Finish"><Icon name="check" size={14} /></button>
    </div>
  {:else}
    <span class="empty" data-tauri-drag-region>{timerStore.error || "No active timer"}</span>
  {/if}
</div>

<style>
  :global(html), :global(body) { background: transparent; }
  .mini { position: relative; display: flex; align-items: center; gap: 10px; height: 100vh; box-sizing: border-box; padding: 0 12px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .close-btn { position: absolute; top: 3px; right: 5px; min-height: 0; width: 16px; height: 16px; padding: 0; border: 0; background: transparent; color: var(--muted); font-size: 13px; line-height: 1; border-radius: 50%; z-index: 1; }
  .close-btn:hover { background: var(--hover); color: var(--text); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #4caf6a; flex-shrink: 0; }
  .dot.paused { background: #c59442; }
  .dot.error { background: var(--danger); }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .info strong { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .clock { font-size: 11px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .actions { display: flex; gap: 4px; }
  .actions button { min-height: 26px; min-width: 26px; padding: 0; font-size: 13px; }
  .empty { color: var(--muted); font-size: 12px; }
</style>
