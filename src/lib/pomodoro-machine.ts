// Pure Pomodoro rules: no timers, storage, or Tauri calls, so they are trivially testable.
import type { PomodoroSettings } from "$lib/api";

export type Phase = "idle" | "work" | "break";
export type ActivePhase = Exclude<Phase, "idle">;

export const CYCLES_PER_LONG_BREAK = 4;
export const STORAGE_KEY = "tasktimer-pomodoro-v02";

export interface PersistedState {
  phase: ActivePhase;
  deadlineMs: number;
  completedWorkCycles: number;
  onLongBreak: boolean;
  pausedSessionId: number | null;
}

export function isLongBreak(phase: ActivePhase, completedWorkCycles: number): boolean {
  return phase === "break" && completedWorkCycles % CYCLES_PER_LONG_BREAK === 0;
}

export function phaseMinutes(settings: PomodoroSettings, phase: ActivePhase, longBreak: boolean): number {
  if (phase === "work") return settings.work_minutes;
  return longBreak ? settings.long_break_minutes : settings.break_minutes;
}

export function secondsRemaining(deadlineMs: number, nowMs: number): number {
  return Math.max(0, Math.ceil((deadlineMs - nowMs) / 1000));
}

export function trayLabel(phase: ActivePhase, longBreak: boolean): string {
  return phase === "work" ? "Focus" : longBreak ? "Long break" : "Break";
}

/** Returns null for missing or malformed data; throws only on invalid JSON. */
export function parsePersisted(raw: string | null): PersistedState | null {
  const saved = JSON.parse(raw ?? "null");
  if (!saved || (saved.phase !== "work" && saved.phase !== "break")) return null;
  if (!Number.isFinite(saved.deadlineMs) || !Number.isInteger(saved.completedWorkCycles)) return null;
  return {
    phase: saved.phase,
    deadlineMs: saved.deadlineMs,
    completedWorkCycles: saved.completedWorkCycles,
    onLongBreak: saved.onLongBreak === true,
    pausedSessionId: Number.isInteger(saved.pausedSessionId) ? saved.pausedSessionId : null,
  };
}
