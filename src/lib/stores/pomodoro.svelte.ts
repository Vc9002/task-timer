import { getPomodoroSettings, notifyPomodoroPhase, setTrayPomodoroStatus, type PomodoroSettings } from "$lib/api";
import { timerStore } from "$lib/stores/timer.svelte";
import { formatHms } from "$lib/format";

type Phase = "idle" | "work" | "break";
const CYCLES_PER_LONG_BREAK = 4;
const STORAGE_KEY = "tasktimer-pomodoro-v02";

class PomodoroStore {
  phase = $state<Phase>("idle");
  remainingSeconds = $state(0);
  onLongBreak = $state(false);
  settings = $state<PomodoroSettings>({ work_minutes: 25, break_minutes: 5, long_break_minutes: 15 });
  private tickHandle: ReturnType<typeof setInterval> | null = null;
  private pausedSessionId: number | null = null;
  private completedWorkCycles = 0;
  private deadlineMs = 0;
  private transitioning = false;

  async loadSettings() {
    try {
      this.settings = await getPomodoroSettings();
    } catch {
      // keep defaults; Settings page surfaces the real error on save
    }
    try {
      const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null");
      if (saved && (saved.phase === "work" || saved.phase === "break") && Number.isFinite(saved.deadlineMs) && Number.isInteger(saved.completedWorkCycles)) {
        this.phase = saved.phase;
        this.deadlineMs = saved.deadlineMs;
        this.completedWorkCycles = saved.completedWorkCycles;
        this.onLongBreak = saved.onLongBreak === true;
        this.pausedSessionId = Number.isInteger(saved.pausedSessionId) ? saved.pausedSessionId : null;
        this.startTicking();
        void this.tick();
      }
    } catch { localStorage.removeItem(STORAGE_KEY); }
  }

  private startTicking() {
    this.tickHandle ??= setInterval(() => void this.tick(), 1000);
  }

  private persist() {
    if (this.phase === "idle") { localStorage.removeItem(STORAGE_KEY); return; }
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ phase: this.phase, deadlineMs: this.deadlineMs, completedWorkCycles: this.completedWorkCycles, onLongBreak: this.onLongBreak, pausedSessionId: this.pausedSessionId }));
  }

  private async releaseOwnedPause(): Promise<boolean> {
    const id = this.pausedSessionId;
    if (id !== null && timerStore.active?.session.id === id && timerStore.active.is_paused) {
      await timerStore.resume();
      if (timerStore.active?.session.id === id && timerStore.active.is_paused) return false;
    }
    this.pausedSessionId = null;
    this.persist();
    return true;
  }

  // timerStore ignores a call while another change is in flight, so retry briefly.
  private async pauseSession(id: number): Promise<boolean> {
    for (let attempt = 0; attempt < 5; attempt++) {
      const active = timerStore.active;
      if (active?.session.id !== id) return false;
      if (active.is_paused) return true;
      await timerStore.pause();
      if (timerStore.active?.session.id === id && timerStore.active.is_paused) return true;
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
    return false;
  }

  private pushTrayStatus() {
    const phaseLabel = this.phase === "work" ? "Focus" : this.onLongBreak ? "Long break" : "Break";
    void setTrayPomodoroStatus(`Pomodoro: ${phaseLabel} ${formatHms(this.remainingSeconds)}`);
  }

  private async tick() {
    if (this.phase === "idle" || this.transitioning) return;
    this.remainingSeconds = Math.max(0, Math.ceil((this.deadlineMs - Date.now()) / 1000));
    if (this.remainingSeconds % 15 === 0) this.pushTrayStatus();
    if (this.remainingSeconds > 0) return;
    await this.transition();
  }

  private async transition() {
    if (this.phase === "idle" || this.transitioning) return;
    this.transitioning = true;
    try {
      if (this.phase === "work") {
        void notifyPomodoroPhase("work_done");
        this.completedWorkCycles += 1;
        if (timerStore.active && !timerStore.active.is_paused) {
          const id = timerStore.active.session.id;
          if (await this.pauseSession(id)) this.pausedSessionId = id;
        }
        this.beginPhase("break");
      } else if (this.phase === "break") {
        if (!await this.releaseOwnedPause()) return;
        this.beginPhase("work");
        void notifyPomodoroPhase("break_done");
      }
      this.pushTrayStatus();
    } finally { this.transitioning = false; }
  }

  private beginPhase(phase: "work" | "break") {
    this.phase = phase;
    this.onLongBreak = phase === "break" && this.completedWorkCycles % CYCLES_PER_LONG_BREAK === 0;
    const minutes = phase === "work"
      ? this.settings.work_minutes
      : this.onLongBreak
        ? this.settings.long_break_minutes
        : this.settings.break_minutes;
    this.remainingSeconds = minutes * 60;
    this.deadlineMs = Date.now() + minutes * 60_000;
    this.persist();
  }

  start() {
    if (this.phase !== "idle") return;
    this.beginPhase("work");
    this.pushTrayStatus();
    this.startTicking();
  }

  async skip() {
    if (this.phase === "idle") return;
    await this.transition();
  }

  async stop() {
    if (this.transitioning) return;
    this.transitioning = true;
    try {
      if (!await this.releaseOwnedPause()) return;
      this.phase = "idle";
      this.remainingSeconds = 0;
      this.completedWorkCycles = 0;
      this.onLongBreak = false;
      this.deadlineMs = 0;
      if (this.tickHandle !== null) clearInterval(this.tickHandle);
      this.tickHandle = null;
      this.persist();
      void setTrayPomodoroStatus(null);
    } finally { this.transitioning = false; }
  }
}

export const pomodoroStore = new PomodoroStore();
