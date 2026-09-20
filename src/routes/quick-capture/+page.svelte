<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { createTask, listClasses, type ClassRecord } from "$lib/api";

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

  function parse(raw: string): { title: string; classId: number | null; estimate: number | null; scheduledToday: boolean } {
    const lower = raw.toLowerCase();
    const matchedClass = classes.find(c => lower.includes(c.course_code.toLowerCase()));
    const durationMatch = raw.match(/(\d+)\s*m(?:in(?:ute)?s?)?/i);
    const scheduledToday = lower.includes("today") || !lower.includes("tomorrow");
    const removable = new RegExp(
      `\\b(?:${classes.map(c => c.course_code.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("|")}|\\d+\\s*m(?:in(?:ute)?s?)?|today|tomorrow)\\b`,
      "gi",
    );
    const title = raw.replace(removable, "").replace(/\s+/g, " ").trim();
    return {
      title,
      classId: matchedClass?.id ?? classes[0]?.id ?? null,
      estimate: durationMatch ? Number(durationMatch[1]) : null,
      scheduledToday,
    };
  }

  async function submit() {
    const raw = shorthand.trim();
    if (!raw || busy) return;
    const { title, classId, estimate, scheduledToday } = parse(raw);
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
        scheduled_date: scheduledToday ? new Date().toISOString().slice(0, 10) : null,
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
  .capture { padding: 14px 16px; background: var(--surface); height: 100vh; box-sizing: border-box; border: 1px solid var(--line); border-radius: 10px; }
  form { display: flex; gap: 8px; }
  input { flex: 1; min-height: 38px; font-size: 14px; }
  button { min-height: 38px; }
  .error { color: var(--danger); font-size: 12px; margin: 8px 0 0; }
  .hint { color: var(--muted); font-size: 11px; margin: 8px 0 0; }
</style>
