<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { createTask, listClasses, type ClassRecord } from "$lib/api";
  import { parseQuickCapture } from "$lib/quickCapture";

  let classes = $state<ClassRecord[]>([]);
  let shorthand = $state("");
  let busy = $state(false);
  let error = $state("");
  let input: HTMLInputElement;

  onMount(() => {
    void listClasses().then(value => classes = value).catch(() => {});
    input?.focus();
    const unlisten = listen("quick-capture-shown", () => {
      shorthand = "";
      error = "";
      input?.focus();
    });
    const onKeydown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void invoke("hide_quick_capture");
    };
    window.addEventListener("keydown", onKeydown);
    return () => {
      void unlisten.then(u => u());
      window.removeEventListener("keydown", onKeydown);
    };
  });

  async function submit() {
    const raw = shorthand.trim();
    if (!raw || busy) return;
    const { title, classId, estimate, scheduledDate } = parseQuickCapture(raw, classes);
    if (!title || classId === null) {
      error = classes.length === 0 ? "Create a class in TaskTimer first." : "Couldn't tell what to add.";
      return;
    }
    busy = true;
    error = "";
    try {
      await createTask({
        class_id: classId,
        parent_task_id: null,
        title,
        description: null,
        priority: null,
        due_at: null,
        scheduled_date: scheduledDate,
        estimated_minutes: estimate,
        task_type: "assignment",
        tags: [],
        time_budget_minutes: null,
      });
      await invoke("hide_quick_capture");
    } catch {
      error = "Couldn't add the task. Try again.";
    } finally {
      busy = false;
    }
  }
</script>

<div class="capture">
  <button
    type="button"
    class="close-btn"
    aria-label="Close"
    title="Close (Esc)"
    onclick={() => void invoke("hide_quick_capture")}
  >&times;</button>
  <form onsubmit={e => { e.preventDefault(); void submit(); }}>
    <input
      bind:this={input}
      bind:value={shorthand}
      disabled={busy}
      placeholder="LGST case brief 60m tomorrow"
      spellcheck="false"
    />
    <button type="submit" disabled={busy || !shorthand.trim()}>{busy ? "Adding…" : "Add"}</button>
  </form>
  {#if error}<p class="error">{error}</p>{/if}
  <p class="hint">Class code + minutes + today/tomorrow, if mentioned · Esc to dismiss</p>
</div>

<style>
  :global(html), :global(body) { background: transparent; }
  .capture { position: relative; padding: 14px 16px; background: var(--surface); height: 100vh; box-sizing: border-box; border: 1px solid var(--line); border-radius: 10px; }
  form { display: flex; gap: 8px; }
  input { flex: 1; min-height: 38px; font-size: 14px; }
  button { min-height: 38px; }
  .error { color: var(--danger); font-size: 12px; margin: 8px 0 0; }
  .hint { color: var(--muted); font-size: 11px; margin: 8px 0 0; }
  .close-btn { position: absolute; top: 6px; right: 8px; min-height: 0; width: 20px; height: 20px; padding: 0; border: 0; background: transparent; color: var(--muted); font-size: 16px; line-height: 1; border-radius: 50%; }
  .close-btn:hover { background: var(--hover); color: var(--text); }
</style>
