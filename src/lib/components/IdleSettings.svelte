<script lang="ts">
  import { onMount } from "svelte";
  import { getIdleThreshold, saveIdleThreshold } from "$lib/api";

  let minutes = $state(5);
  let error = $state("");
  let saved = $state(false);

  onMount(async () => {
    try {
      minutes = await getIdleThreshold();
    } catch (e) {
      error = String(e);
    }
  });

  async function save() {
    try {
      await saveIdleThreshold(minutes);
      error = "";
      saved = true;
      setTimeout(() => saved = false, 1500);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section>
  <h2>Idle auto-pause</h2>
  <p class="muted">Automatically pause the active timer after this many minutes without keyboard or mouse activity. Set to 0 to turn off.</p>
  {#if error}<p class="error">{error}</p>{/if}
  <label class="idle-field">Pause after <input type="number" min="0" max="120" bind:value={minutes} /> min idle</label>
  <button onclick={save}>{saved ? "Saved" : "Save idle setting"}</button>
</section>

<style>
  .idle-field { display: flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); margin: 12px 0; }
  .idle-field input { border: 1px solid var(--line); border-radius: 6px; padding: 5px 8px; font-size: 13px; width: 64px; }
</style>
