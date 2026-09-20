<script lang="ts">
  import { timerStore } from "$lib/stores/timer.svelte";
  import { formatHms } from "$lib/format";
</script>

<div class="mini" data-tauri-drag-region>
  {#if timerStore.active}
    <span class="dot" class:paused={timerStore.active.is_paused} data-tauri-drag-region></span>
    <div class="info" data-tauri-drag-region>
      <strong>{timerStore.active.task_title}</strong>
      <span class="clock">{formatHms(timerStore.displaySeconds)}</span>
    </div>
    <div class="actions">
      <button
        onclick={() => timerStore.active?.is_paused ? timerStore.resume() : timerStore.pause()}
        title={timerStore.active.is_paused ? "Resume" : "Pause"}
      >{timerStore.active.is_paused ? "▶" : "⏸"}</button>
      <button onclick={() => timerStore.finish()} title="Finish">✓</button>
    </div>
  {:else}
    <span class="empty" data-tauri-drag-region>No active timer</span>
  {/if}
</div>

<style>
  :global(html), :global(body) { background: transparent; }
  .mini { display: flex; align-items: center; gap: 10px; height: 100vh; box-sizing: border-box; padding: 0 12px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: #4caf6a; flex-shrink: 0; }
  .dot.paused { background: #c59442; }
  .info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 1px; }
  .info strong { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .clock { font-size: 11px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .actions { display: flex; gap: 4px; }
  .actions button { min-height: 26px; min-width: 26px; padding: 0; font-size: 13px; }
  .empty { color: var(--muted); font-size: 12px; }
</style>
