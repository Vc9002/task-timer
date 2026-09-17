import { getPomodoroSettings, notifyPomodoroPhase, setTrayPomodoroStatus, type PomodoroSettings } from "$lib/api";
import { timerStore } from "$lib/stores/timer.svelte";
import { formatHms } from "$lib/format";

type Phase = "idle" | "work" | "break";
const CYCLES_PER_LONG_BREAK = 4;

class PomodoroStore {
  phase = $state<Phase>("idle");
  remainingSeconds = $state(0);
  onLongBreak = $state(false);
  settings = $state<PomodoroSettings>({ work_minutes: 25, break_minutes: 5, long_break_minutes: 15 });
  private tickHandle: ReturnType<typeof setInterval> | null = null;
  private pausedByPomodoro = false;
  private completedWorkCycles = 0;

  async loadSettings() {
    try {
      this.settings = await getPomodoroSettings();
    } catch {
      // keep defaults; Settings page surfaces the real error on save
    }
  }

  private pushTrayStatus() {
    const phaseLabel = this.phase === "work" ? "Focus" : this.onLongBreak ? "Long break" : "Break";
    void setTrayPomodoroStatus(`Pomodoro: ${phaseLabel} ${formatHms(this.remainingSeconds)}`);
  }

  private tick() {
    this.remainingSeconds -= 1;
    if (this.remainingSeconds % 15 === 0) this.pushTrayStatus();
    if (this.remainingSeconds > 0) return;
    if (this.phase === "work") {
      void notifyPomodoroPhase("work_done");
      this.completedWorkCycles += 1;
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
    this.onLongBreak = phase === "break" && this.completedWorkCycles % CYCLES_PER_LONG_BREAK === 0;
    const minutes = phase === "work"
      ? this.settings.work_minutes
      : this.onLongBreak
        ? this.settings.long_break_minutes
        : this.settings.break_minutes;
    this.remainingSeconds = minutes * 60;
  }

  start() {
    if (this.phase !== "idle") return;
    this.beginPhase("work");
    this.pushTrayStatus();
    this.tickHandle ??= setInterval(() => this.tick(), 1000);
  }

  skip() {
    if (this.phase === "idle") return;
    if (this.phase === "work") this.completedWorkCycles += 1;
    this.beginPhase(this.phase === "work" ? "break" : "work");
    this.pushTrayStatus();
  }

  stop() {
    this.phase = "idle";
    this.remainingSeconds = 0;
    this.pausedByPomodoro = false;
    this.completedWorkCycles = 0;
    this.onLongBreak = false;
    if (this.tickHandle !== null) clearInterval(this.tickHandle);
    this.tickHandle = null;
    void setTrayPomodoroStatus(null);
  }
}

export const pomodoroStore = new PomodoroStore();
