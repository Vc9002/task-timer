import { beforeEach, afterEach, describe, expect, it, vi } from "vitest";

const timer = vi.hoisted(() => ({
  active: null as null | { session: { id: number }; is_paused: boolean },
  pause: vi.fn(),
  resume: vi.fn(),
}));

vi.mock("$lib/api", () => ({
  getPomodoroSettings: vi.fn(async () => ({ work_minutes: 25, break_minutes: 5, long_break_minutes: 15 })),
  notifyPomodoroPhase: vi.fn(async () => {}),
  setTrayPomodoroStatus: vi.fn(async () => {}),
}));
vi.mock("$lib/stores/timer.svelte", () => ({ timerStore: timer }));

const KEY = "tasktimer-pomodoro-v02";
const MIN = 60_000;

function stubStorage() {
  const data = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (k: string) => data.get(k) ?? null,
    setItem: (k: string, v: string) => void data.set(k, v),
    removeItem: (k: string) => void data.delete(k),
  });
  return data;
}

async function freshStore() {
  vi.resetModules();
  return (await import("./pomodoro.svelte")).pomodoroStore;
}

describe("pomodoroStore", () => {
  let storage: Map<string, string>;

  beforeEach(() => {
    vi.useFakeTimers();
    storage = stubStorage();
    timer.active = null;
    timer.pause.mockReset().mockImplementation(async () => { if (timer.active) timer.active.is_paused = true; });
    timer.resume.mockReset().mockImplementation(async () => { if (timer.active) timer.active.is_paused = false; });
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("starts a work phase and persists it", async () => {
    const store = await freshStore();
    store.start();
    expect(store.phase).toBe("work");
    expect(store.remainingSeconds).toBe(25 * 60);
    expect(JSON.parse(storage.get(KEY)!).phase).toBe("work");
    await store.stop();
  });

  it("moves to a break when the work deadline passes and pauses the task timer", async () => {
    timer.active = { session: { id: 7 }, is_paused: false };
    const store = await freshStore();
    store.start();
    await vi.advanceTimersByTimeAsync(25 * MIN + 1000);
    expect(store.phase).toBe("break");
    expect(timer.pause).toHaveBeenCalledTimes(1);
    expect(JSON.parse(storage.get(KEY)!).pausedSessionId).toBe(7);
    await store.stop();
  });

  it("resumes only the session it paused when the break ends", async () => {
    timer.active = { session: { id: 7 }, is_paused: false };
    const store = await freshStore();
    store.start();
    await vi.advanceTimersByTimeAsync(25 * MIN + 1000);
    await vi.advanceTimersByTimeAsync(5 * MIN + 1000);
    expect(store.phase).toBe("work");
    expect(timer.resume).toHaveBeenCalledTimes(1);
    expect(timer.active.is_paused).toBe(false);
    await store.stop();
  });

  it("does not resume a timer the user paused themselves", async () => {
    timer.active = { session: { id: 7 }, is_paused: true };
    const store = await freshStore();
    store.start();
    await vi.advanceTimersByTimeAsync(25 * MIN + 1000);
    await vi.advanceTimersByTimeAsync(5 * MIN + 1000);
    expect(timer.pause).not.toHaveBeenCalled();
    expect(timer.resume).not.toHaveBeenCalled();
    await store.stop();
  });

  it("retries the pause when the timer store is busy", async () => {
    timer.active = { session: { id: 3 }, is_paused: false };
    let calls = 0;
    timer.pause.mockImplementation(async () => { if (++calls >= 3) timer.active!.is_paused = true; });
    const store = await freshStore();
    store.start();
    await vi.advanceTimersByTimeAsync(25 * MIN + 1000);
    expect(calls).toBe(3);
    expect(JSON.parse(storage.get(KEY)!).pausedSessionId).toBe(3);
    await store.stop();
  });

  it("takes a long break after four work cycles", async () => {
    const store = await freshStore();
    store.start();
    for (let i = 0; i < 3; i++) {
      await vi.advanceTimersByTimeAsync(25 * MIN + 1000);
      expect(store.onLongBreak).toBe(false);
      await vi.advanceTimersByTimeAsync(5 * MIN + 1000);
    }
    await vi.advanceTimersByTimeAsync(25 * MIN + 1000);
    expect(store.onLongBreak).toBe(true);
    expect(store.remainingSeconds).toBeGreaterThan(14 * 60);
    await store.stop();
  });

  it("restores a persisted phase and ignores corrupt data", async () => {
    storage.set(KEY, JSON.stringify({ phase: "work", deadlineMs: Date.now() + 10 * MIN, completedWorkCycles: 1, onLongBreak: false, pausedSessionId: null }));
    const store = await freshStore();
    await store.loadSettings();
    expect(store.phase).toBe("work");
    expect(store.remainingSeconds).toBe(10 * 60);
    await store.stop();

    storage.set(KEY, "{not json");
    const other = await freshStore();
    await other.loadSettings();
    expect(other.phase).toBe("idle");
    expect(storage.has(KEY)).toBe(false);
  });

  it("stop clears persisted state and returns to idle", async () => {
    const store = await freshStore();
    store.start();
    await store.stop();
    expect(store.phase).toBe("idle");
    expect(storage.has(KEY)).toBe(false);
  });
});
