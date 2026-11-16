import { describe, expect, it } from "vitest";
import { formatDue } from "./format";

describe("formatDue", () => {
  it("keeps a date-only due date on its own calendar day", () => {
    expect(formatDue("2026-11-15", "en-US")).toBe("Nov 15");
    expect(formatDue("2026-01-01", "en-US")).toBe("Jan 1");
  });
  it("reads a zoneless timestamp as UTC, like the backend, and honours explicit zones", () => {
    const expected = (iso: string) => new Date(iso).toLocaleDateString("en-US", { month: "short", day: "numeric" });
    expect(formatDue("2026-11-15T03:00:00", "en-US")).toBe(expected("2026-11-15T03:00:00Z"));
    expect(formatDue("2026-11-15 03:00:00", "en-US")).toBe(expected("2026-11-15T03:00:00Z"));
    expect(formatDue("2026-11-15T03:00:00Z", "en-US")).toBe(expected("2026-11-15T03:00:00Z"));
    expect(formatDue("2026-11-15T23:30:00-05:00", "en-US")).toBe(expected("2026-11-16T04:30:00Z"));
  });
  it("falls back to the raw date for unparseable input", () => {
    expect(formatDue("not-a-date-at-all", "en-US")).toBe("not-a-date");
  });
});
