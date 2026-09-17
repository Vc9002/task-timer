<script lang="ts">
  import { onMount } from "svelte";
  import { getWeeklyReviewSettings, saveWeeklyReviewSettings } from "$lib/api";

  const weekdayLabels = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
  let enabled = $state(false);
  let weekday = $state(0);
  let hour = $state(18);
  let error = $state("");
  let saved = $state(false);

  onMount(async () => {
    try {
      const settings = await getWeeklyReviewSettings();
      enabled = settings.enabled;
      weekday = settings.weekday;
      hour = settings.hour;
    } catch (e) {
      error = String(e);
    }
  });

  async function save() {
    try {
      await saveWeeklyReviewSettings({ enabled, weekday, hour });
      error = "";
      saved = true;
      setTimeout(() => saved = false, 1500);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section>
  <h2>Weekly review nudge</h2>
  <p class="muted">A local notification pointing at the Analytics weekly review, once a week.</p>
  {#if error}<p class="error">{error}</p>{/if}
  <label class="nudge-toggle"><input type="checkbox" bind:checked={enabled} /> Remind me</label>
  <div class="nudge-fields">
    <label>Day <select bind:value={weekday}>{#each weekdayLabels as label, i}<option value={i}>{label}</option>{/each}</select></label>
    <label>Hour <select bind:value={hour}>{#each Array.from({ length: 24 }, (_, i) => i) as h}<option value={h}>{h}:00</option>{/each}</select></label>
  </div>
  <button onclick={save}>{saved ? "Saved" : "Save weekly review setting"}</button>
</section>

<style>
  .nudge-toggle { display: flex; align-items: center; gap: 8px; font-size: 13px; margin: 12px 0; }
  .nudge-fields { display: flex; gap: 20px; margin: 12px 0; }
  .nudge-fields label { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); }
</style>
