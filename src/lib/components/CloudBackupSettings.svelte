<script lang="ts">
  import { onMount } from "svelte";
  import { getCloudBackupSettings, saveCloudBackupSettings, pickCloudBackupFolder, runCloudBackupNow } from "$lib/api";

  let enabled = $state(false);
  let folder = $state<string | null>(null);
  let error = $state("");
  let saved = $state(false);
  let backingUp = $state(false);
  let lastBackupPath = $state("");

  onMount(async () => {
    try {
      const settings = await getCloudBackupSettings();
      enabled = settings.enabled;
      folder = settings.folder;
    } catch (e) {
      error = String(e);
    }
  });

  async function chooseFolder() {
    try {
      const picked = await pickCloudBackupFolder();
      if (picked) folder = picked;
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    try {
      await saveCloudBackupSettings({ enabled, folder });
      error = "";
      saved = true;
      setTimeout(() => saved = false, 1500);
    } catch (e) {
      error = String(e);
    }
  }

  async function backupNow() {
    backingUp = true;
    try {
      lastBackupPath = await runCloudBackupNow();
      error = "";
    } catch (e) {
      error = String(e);
    } finally {
      backingUp = false;
    }
  }
</script>

<section>
  <h2>Auto backup to folder</h2>
  <p class="muted">Writes a timestamped backup snapshot into a folder you choose (e.g. inside iCloud Drive or Dropbox) once a day, keeping the most recent 30.</p>
  {#if error}<p class="error">{error}</p>{/if}
  <label class="backup-toggle"><input type="checkbox" bind:checked={enabled} /> Back up automatically</label>
  <div class="backup-folder">
    <input readonly value={folder ?? "No folder chosen"} />
    <button type="button" onclick={chooseFolder}>Choose folder…</button>
  </div>
  <div class="backup-actions">
    <button onclick={save}>{saved ? "Saved" : "Save backup setting"}</button>
    <button type="button" class="quiet" disabled={backingUp || !folder} onclick={backupNow}>{backingUp ? "Backing up…" : "Back up now"}</button>
  </div>
  {#if lastBackupPath}<p class="muted small">Saved to {lastBackupPath}</p>{/if}
</section>

<style>
  .backup-toggle { display: flex; align-items: center; gap: 8px; font-size: 13px; margin: 12px 0; }
  .backup-folder { display: flex; gap: 8px; margin: 12px 0; }
  .backup-folder input { flex: 1; color: var(--muted); }
  .backup-actions { display: flex; gap: 8px; }
  .small { font-size: 11px; margin-top: 8px; }
</style>
