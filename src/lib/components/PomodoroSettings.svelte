<script lang="ts">
  import { onMount } from "svelte";
  import { getPomodoroSettings, savePomodoroSettings } from "$lib/api";
  import { pomodoroStore } from "$lib/stores/pomodoro.svelte";

  let workMinutes = $state(25);
  let breakMinutes = $state(5);
  let longBreakMinutes = $state(15);
  let error = $state("");
  let saved = $state(false);

  onMount(async () => {
    try {
      const settings = await getPomodoroSettings();
      workMinutes = settings.work_minutes;
      breakMinutes = settings.break_minutes;
      longBreakMinutes = settings.long_break_minutes;
    } catch (e) {
      error = String(e);
    }
  });

  async function save() {
    try {
      const settings = { work_minutes: workMinutes, break_minutes: breakMinutes, long_break_minutes: longBreakMinutes };
      await savePomodoroSettings(settings);
      pomodoroStore.settings = settings;
      error = "";
      saved = true;
      setTimeout(() => saved = false, 1500);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section>
  <h2>Pomodoro</h2>
  <p class="muted">Set your work, break, and long-break lengths. Every 4th work session takes the long break. Start it anytime from the sidebar, for whatever task you're on.</p>
  {#if error}<p class="error">{error}</p>{/if}
  <div class="pomodoro-grid">
    <label>Work <input type="number" min="1" max="180" bind:value={workMinutes} /> min</label>
    <label>Break <input type="number" min="1" max="60" bind:value={breakMinutes} /> min</label>
    <label>Long break <input type="number" min="1" max="90" bind:value={longBreakMinutes} /> min</label>
  </div>
  <button onclick={save}>{saved ? "Saved" : "Save Pomodoro settings"}</button>
</section>

<style>
  .pomodoro-grid { display: flex; gap: 20px; margin: 12px 0; flex-wrap: wrap; }
  .pomodoro-grid label { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); }
  .pomodoro-grid input { border: 1px solid var(--line); border-radius: 6px; padding: 5px 8px; font-size: 13px; width: 64px; }
</style>
