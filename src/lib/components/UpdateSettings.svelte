<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { updaterStore } from "$lib/stores/updater.svelte";

  let version = $state("");

  onMount(() => {
    getVersion().then((v) => (version = v)).catch(() => {});
  });
</script>

<section>
  <h2>Updates</h2>
  <p class="muted">Current version {version || "…"}. TaskTimer also checks for updates when it starts.</p>
  <button class="secondary" onclick={() => updaterStore.checkNow()} disabled={updaterStore.checking || updaterStore.installing}>
    {updaterStore.checking ? "Checking…" : "Check for updates"}
  </button>
  {#if updaterStore.notice}<p class="muted">{updaterStore.notice}</p>{/if}
  {#if updaterStore.error}<p class="error">Update check failed: {updaterStore.error}</p>{/if}
</section>
