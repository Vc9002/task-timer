import { describe, expect, it } from "vitest";
import { formatDue } from "./format";

describe("formatDue", () => {
  it("keeps a date-only due date on its own calendar day", () => {
    expect(formatDue("2026-11-15", "en-US")).toBe("Nov 15");
    expect(formatDue("2026-01-01", "en-US")).toBe("Jan 1");
  });
  it("falls back to the raw date for unparseable input", () => {
    expect(formatDue("not-a-date-at-all", "en-US")).toBe("not-a-date");
  });
});
