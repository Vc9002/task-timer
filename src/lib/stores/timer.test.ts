import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const session = { session: { id: 1 }, task_title: "Read", class_course_code: "LGST", is_paused: true, elapsed_seconds: 10 };
const api = vi.hoisted(() => ({ pauseTimer: vi.fn(), getActiveSession: vi.fn() }));
vi.mock("$lib/api", () => api);

async function freshStore() {
  vi.resetModules();
  return (await import("./timer.svelte")).timerStore;
}

describe("timerStore revision", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    api.getActiveSession.mockReset().mockResolvedValue(session);
  });
  afterEach(() => vi.useRealTimers());

  it("bumps once when the backend's change event arrives before the command resolves", async () => {
    const store = await freshStore();
    let resolve!: (value: unknown) => void;
    api.pauseTimer.mockReset().mockReturnValue(new Promise(r => { resolve = r; }));
    const pending = store.pause();
    store.externalChange(); // the echo of our own command, while still busy
    resolve(session);
    await pending;
    await vi.runAllTimersAsync();
    expect(store.revision).toBe(1);
  });

  it("bumps once when the change event arrives just after the command resolves", async () => {
    const store = await freshStore();
    api.pauseTimer.mockReset().mockResolvedValue(session);
    await store.pause();
    store.externalChange();
    await vi.runAllTimersAsync();
    expect(store.revision).toBe(1);
  });

  it("bumps and refreshes for a change made elsewhere, such as the tray", async () => {
    const store = await freshStore();
    store.externalChange();
    await vi.runAllTimersAsync();
    expect(store.revision).toBe(1);
    expect(api.getActiveSession).toHaveBeenCalledTimes(1);
    store.externalChange();
    await vi.runAllTimersAsync();
    expect(store.revision).toBe(2);
  });
});
