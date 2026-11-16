import { describe, expect, it } from "vitest";
import { isLongBreak, parsePersisted, phaseMinutes, secondsRemaining, trayLabel } from "./pomodoro-machine";

const settings = { work_minutes: 25, break_minutes: 5, long_break_minutes: 15 };

describe("pomodoro-machine", () => {
  it("picks a long break every fourth cycle only during breaks", () => {
    expect(isLongBreak("break", 4)).toBe(true);
    expect(isLongBreak("break", 3)).toBe(false);
    expect(isLongBreak("work", 4)).toBe(false);
  });

  it("maps phases to configured minutes", () => {
    expect(phaseMinutes(settings, "work", false)).toBe(25);
    expect(phaseMinutes(settings, "break", false)).toBe(5);
    expect(phaseMinutes(settings, "break", true)).toBe(15);
  });

  it("rounds remaining time up and never goes negative", () => {
    expect(secondsRemaining(10_500, 10_000)).toBe(1);
    expect(secondsRemaining(10_000, 10_000)).toBe(0);
    expect(secondsRemaining(5_000, 10_000)).toBe(0);
  });

  it("labels phases for the tray", () => {
    expect(trayLabel("work", false)).toBe("Focus");
    expect(trayLabel("break", true)).toBe("Long break");
    expect(trayLabel("break", false)).toBe("Break");
  });

  it("parses valid state and rejects malformed state", () => {
    const ok = parsePersisted(JSON.stringify({ phase: "break", deadlineMs: 1, completedWorkCycles: 2, onLongBreak: true, pausedSessionId: 9 }));
    expect(ok).toEqual({ phase: "break", deadlineMs: 1, completedWorkCycles: 2, onLongBreak: true, pausedSessionId: 9 });
    expect(parsePersisted(null)).toBeNull();
    expect(parsePersisted(JSON.stringify({ phase: "idle", deadlineMs: 1, completedWorkCycles: 0 }))).toBeNull();
    expect(parsePersisted(JSON.stringify({ phase: "work", deadlineMs: "x", completedWorkCycles: 0 }))).toBeNull();
    expect(parsePersisted(JSON.stringify({ phase: "work", deadlineMs: 1, completedWorkCycles: 0 }))?.pausedSessionId).toBeNull();
    expect(() => parsePersisted("{bad")).toThrow();
  });
});
