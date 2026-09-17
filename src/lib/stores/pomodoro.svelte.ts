import { getPomodoroSettings, notifyPomodoroPhase, type PomodoroSettings } from "$lib/api";
import { timerStore } from "$lib/stores/timer.svelte";

type Phase = "idle" | "work" | "break";

class PomodoroStore {
  phase = $state<Phase>("idle");
  remainingSeconds = $state(0);
  settings = $state<PomodoroSettings>({ work_minutes: 25, break_minutes: 5 });
  private tickHandle: ReturnType<typeof setInterval> | null = null;
  private pausedByPomodoro = false;

  async loadSettings() {
    try {
      this.settings = await getPomodoroSettings();
    } catch {
      // keep defaults; Settings page surfaces the real error on save
    }
  }

  private tick() {
    this.remainingSeconds -= 1;
    if (this.remainingSeconds > 0) return;
    if (this.phase === "work") {
      void notifyPomodoroPhase("work_done");
      if (timerStore.active && !timerStore.active.is_paused) {
        this.pausedByPomodoro = true;
        void timerStore.pause();
      }
      this.beginPhase("break");
    } else if (this.phase === "break") {
      void notifyPomodoroPhase("break_done");
      if (this.pausedByPomodoro && timerStore.active?.is_paused) {
        void timerStore.resume();
      }
      this.pausedByPomodoro = false;
      this.beginPhase("work");
    }
  }

  private beginPhase(phase: "work" | "break") {
    this.phase = phase;
    this.remainingSeconds = (phase === "work" ? this.settings.work_minutes : this.settings.break_minutes) * 60;
  }

  start() {
    if (this.phase !== "idle") return;
    this.beginPhase("work");
    this.tickHandle ??= setInterval(() => this.tick(), 1000);
  }

  skip() {
    if (this.phase === "idle") return;
    this.beginPhase(this.phase === "work" ? "break" : "work");
  }

  stop() {
    this.phase = "idle";
    this.remainingSeconds = 0;
    this.pausedByPomodoro = false;
    if (this.tickHandle !== null) clearInterval(this.tickHandle);
    this.tickHandle = null;
  }
}

export const pomodoroStore = new PomodoroStore();
