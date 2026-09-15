import {
  getActiveSession, startTimer, pauseTimer, resumeTimer, finishTimer,
  cancelTimer, switchTimer, recoverTimer, type ActiveSessionInfo, type TimerError,
} from "$lib/api";

class TimerStore {
  active = $state<ActiveSessionInfo | null>(null);
  displaySeconds = $state(0);
  busy = $state(false);
  error = $state("");
  recovery = $state(false);
  revision = $state(0);
  conflict = $state<{ taskId: number; taskTitle: string; classCode: string; requestedId: number; requestedTitle: string; sessionId: number } | null>(null);
  private fetchedAtMs = 0;
  private tickHandle: ReturnType<typeof setInterval> | null = null;
  private request = 0;
  private disposed = false;

  private applyActive(info: ActiveSessionInfo | null) {
    this.active = info;
    this.fetchedAtMs = Date.now();
    this.displaySeconds = info?.elapsed_seconds ?? 0;
    if (this.tickHandle !== null) clearInterval(this.tickHandle);
    this.tickHandle = null;
    if (info && !info.is_paused && !this.disposed) {
      this.tickHandle = setInterval(() => {
        this.displaySeconds = Math.max(0, info.elapsed_seconds + (Date.now() - this.fetchedAtMs) / 1000);
      }, 1000);
    }
  }

  async refresh(startup = false) {
    if (this.busy) return;
    const request = ++this.request;
    try {
      const info = await getActiveSession();
      if (request !== this.request || this.disposed) return;
      this.applyActive(info);
      if (startup) this.recovery = info !== null;
      if (!info) { this.recovery = false; this.conflict = null; }
      this.error = "";
    } catch { this.error = "Couldn't refresh the timer. Try again."; }
  }

  private async change(action: () => Promise<void>) {
    if (this.busy) return;
    this.busy = true;
    ++this.request;
    this.error = "";
    try { await action(); this.revision++; }
    catch { this.error = "TaskTimer couldn't save the timer change. Try again."; }
    finally { this.busy = false; }
  }

  async start(taskId: number, requestedTitle = "Requested task"): Promise<boolean> {
    let started = false;
    await this.change(async () => {
      this.conflict = null;
      try {
        this.applyActive(await startTimer(taskId));
        started = true;
      } catch (e) {
        const err = e as TimerError;
        if (err?.kind !== "ActiveSessionConflict") throw e;
        const current = await getActiveSession();
        this.applyActive(current);
        if (current) this.conflict = {
          taskId: err.task_id, taskTitle: err.task_title, classCode: err.class_course_code,
          requestedId: taskId, requestedTitle, sessionId: current.session.id,
        };
      }
    });
    return started;
  }

  async switchToRequested() {
    const conflict = this.conflict;
    if (!conflict) return;
    await this.change(async () => {
      this.applyActive(await switchTimer(conflict.sessionId, conflict.requestedId));
      this.conflict = null;
      this.recovery = false;
    });
  }
  async pause() { await this.change(async () => this.applyActive(await pauseTimer())); }
  async resume() { await this.change(async () => this.applyActive(await resumeTimer())); }
  async finish() {
    await this.change(async () => { await finishTimer(); this.applyActive(null); this.recovery = false; });
  }
  async cancel() {
    await this.change(async () => { await cancelTimer(); this.applyActive(null); this.recovery = false; });
  }
  async finishEdited(minutes: number) {
    if (!this.active || !Number.isFinite(minutes) || minutes < 0) return;
    const id = this.active.session.id;
    await this.change(async () => {
      await recoverTimer(id, Math.round(minutes * 60));
      this.applyActive(null); this.recovery = false;
    });
  }
  mount() { this.disposed = false; void this.refresh(true); }
  destroy() {
    this.disposed = true;
    ++this.request;
    if (this.tickHandle !== null) clearInterval(this.tickHandle);
    this.tickHandle = null;
  }
}
export const timerStore = new TimerStore();
