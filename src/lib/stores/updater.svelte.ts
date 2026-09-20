import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

class UpdaterStore {
  ready = $state(false);
  installing = $state(false);
  error = $state<string | null>(null);
  private pending: Update | null = null;

  async checkInBackground() {
    try {
      const update = await check();
      if (!update) return;
      await update.downloadAndInstall();
      this.pending = update;
      this.ready = true;
    } catch (error) {
      this.error = error instanceof Error ? error.message : String(error);
    }
  }

  async restartNow() {
    this.installing = true;
    try {
      await relaunch();
    } catch (error) {
      this.installing = false;
      this.error = error instanceof Error ? error.message : String(error);
    }
  }

  dismiss() {
    this.ready = false;
  }
}

export const updaterStore = new UpdaterStore();
