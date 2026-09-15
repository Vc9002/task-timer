import {
  getActiveSession,
  startTimer,
  pauseTimer,
  resumeTimer,
  finishTimer,
  cancelTimer,
  type ActiveSessionInfo,
  type TimerError,
} from "$lib/api";

class TimerStore {
  active = $state<ActiveSessionInfo | null>(null);
  /** ms performance clock captured at the moment `active` was last fetched. */
  private fetchedAtMs = 0;
  displaySeconds = $state(0);
  conflict = $state<{ taskId: number; taskTitle: string; classCode: string } | null>(null);
  private tickHandle: ReturnType<typeof setInterval> | null = null;

  constructor() {
    this.tickHandle = setInterval(() => this.tick(), 1000);
  }

  private applyActive(info: ActiveSessionInfo | null) {
    this.active = info;
    this.fetchedAtMs = performance.now();
    this.displaySeconds = info?.elapsed_seconds ?? 0;
  }

  private tick() {
    if (!this.active || this.active.is_paused) return;
    // Interpolate between authoritative fetches using a real elapsed-ms delta,
    // never a naive per-tick increment — self-corrects on the next fetch.
    const deltaSeconds = (performance.now() - this.fetchedAtMs) / 1000;
    this.displaySeconds = this.active.elapsed_seconds + deltaSeconds;
  }

  async refresh() {
    const info = await getActiveSession();
    this.applyActive(info);
  }

  async start(taskId: number): Promise<boolean> {
    this.conflict = null;
    try {
      const info = await startTimer(taskId);
      this.applyActive(info);
      return true;
    } catch (e) {
      const err = e as TimerError;
      if (err && typeof err === "object" && "kind" in err && err.kind === "ActiveSessionConflict") {
        this.conflict = {
          taskId: err.task_id,
          taskTitle: err.task_title,
          classCode: err.class_course_code,
        };
      }
      return false;
    }
  }

  async pause() {
    const info = await pauseTimer();
    this.applyActive(info);
  }

  async resume() {
    const info = await resumeTimer();
    this.applyActive(info);
  }

  async finish() {
    await finishTimer();
    this.applyActive(null);
  }

  async cancel() {
    await cancelTimer();
    this.applyActive(null);
  }

  destroy() {
    if (this.tickHandle) clearInterval(this.tickHandle);
  }
}

export const timerStore = new TimerStore();
