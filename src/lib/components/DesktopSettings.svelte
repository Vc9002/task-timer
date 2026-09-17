<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { exportData, notificationSettings, saveNotificationSettings, testNotification } from "$lib/api";
  type Settings = { close_to_tray: boolean; start_hidden: boolean; shortcuts: string[] };
  let settings = $state<Settings | null>(null);
  let autostart = $state(false);
  let busy = $state(false);
  let message = $state("");
  let warning = $state<string | null>(null);
  let notifications = $state({ enabled: false, overrun_percent: 25 });
  async function load() {
    const status = await invoke<{ settings: Settings; autostart: boolean; shortcut_warning: string | null }>("desktop_status");
    settings = status.settings; autostart = status.autostart; warning = status.shortcut_warning; notifications = await notificationSettings();
  }
  async function saveNotifications() { busy = true; message = ""; try { await saveNotificationSettings(notifications); message = "Notification settings saved."; } catch (error) { message = String(error); } finally { busy = false; } }
  async function exportFile(format: "csv" | "tasks" | "json") {
    busy = true; message = "";
    try { message = await exportData(format) ? "Export saved." : "Export cancelled."; }
    catch (error) { message = String(error); }
    finally { busy = false; }
  }
  async function sendTest() {
    busy = true; message = "";
    try { await testNotification(); message = "Test notification sent. If it doesn't appear, check system notification settings."; }
    catch (error) { message = String(error); }
    finally { busy = false; }
  }
  onMount(() => { void load().catch(() => message = "Couldn't load desktop settings."); });
  async function save() {
    busy = true; message = "";
    try { await invoke("save_desktop_settings", { settings }); await load(); message = "Desktop settings saved."; }
    catch (error) { message = String(error); }
    finally { busy = false; }
  }
  async function login(enabled: boolean) {
    busy = true; message = "";
    try { await invoke("set_autostart", { enabled }); await load(); }
    catch (error) { message = String(error); }
    finally { busy = false; }
  }
</script>
<section>
  <h2>Desktop</h2>
  {#if settings}
    <form onsubmit={e => { e.preventDefault(); void save(); }}>
      <fieldset disabled={busy}>
        <label>Close button behavior <select bind:value={settings.close_to_tray}><option value={true}>Hide to menu bar / system tray</option><option value={false}>Quit TaskTimer</option></select></label>
        <p>Use Quit TaskTimer in the tray menu to exit completely.</p>
        <label><input type="checkbox" checked={autostart} onchange={e => login(e.currentTarget.checked)} /> Start TaskTimer when I log in</label>
        <label><input type="checkbox" bind:checked={settings.start_hidden} /> Start hidden when launched at login</label>
        <h3>Global shortcuts</h3>
        {#each ["Start / Switch Task", "Pause / Resume", "Finish Timer"] as label, i}
          <label>{label}<input bind:value={settings.shortcuts[i]} spellcheck="false" /></label>
        {/each}
        <p>Use CommandOrControl+Shift+Y format. Start / Switch defaults to Cmd/Ctrl+Shift+Y so Chrome can keep Cmd/Ctrl+Shift+T for reopening a closed tab. Leave blank to disable an action. Cmd/Ctrl+K opens Quick Add inside TaskTimer.</p>
        <button type="submit">Save desktop settings</button>
      </fieldset>
    </form>
  {/if}
  {#if warning}<p role="alert">{warning}</p>{/if}
  <h3>Notifications</h3>
  <label><input type="checkbox" bind:checked={notifications.enabled} /> Timer estimate overrun</label>
  <label>Notify after <select bind:value={notifications.overrun_percent}><option value={0}>100% of estimate</option><option value={25}>125% of estimate</option><option value={50}>150% of estimate</option></select></label>
  <button disabled={busy} onclick={() => saveNotifications()}>Save notifications</button>
  <button disabled={!notifications.enabled || busy} onclick={sendTest}>Send test notification</button>
  <p>Save before testing. Your operating system may block notifications. Each threshold is sent once per session. Failed submissions retry on the next timer change, focus, or launch.</p>
  <h3>Export</h3>
  <button disabled={busy} onclick={() => exportFile("tasks")}>Export tasks CSV</button>
  <button disabled={busy} onclick={() => exportFile("csv")}>Export history CSV</button>
  <button disabled={busy} onclick={() => exportFile("json")}>Export all data JSON</button>
  <p>JSON preserves classes, task hierarchy, time history, and project mappings. Credentials and settings are excluded. Restore/import is not available yet.</p>
  {#if message}<p role="status">{message}</p>{/if}
</section>
<style>
section { margin-bottom: 2rem; }
  fieldset { border: 0; padding: 0; margin: 0; }
  h2, h3 { border-top: 1px solid var(--line); padding-top: 20px; margin-top: 25px; }
  label { display: block; margin: 14px 0; font-size: 12px; }
  label:has(input[type=checkbox]) { display: flex; align-items: center; gap: 8px; }
  input:not([type=checkbox]), select { display: block; margin-top: 6px; width: 100%; max-width: 360px; }
  p { color: var(--muted); font-size: 11px; max-width: 44rem; line-height: 1.7; }
  button { margin: 3px 4px 3px 0; font-size: 12px; }
  [role=status] { padding: 10px 12px; background: var(--accent-soft); color: var(--accent); border-radius: 6px; position: sticky; bottom: 10px; }
</style>
