import { describe, expect, it } from "vitest";
import { parseQuickCapture } from "./quickCapture";

const classes = [{ id: 1, course_code: "LGST 1000" }, { id: 2, course_code: "CS101" }];

describe("parseQuickCapture", () => {
  it("extracts class, estimate and title", () => {
    const r = parseQuickCapture("CS101 problem set 90m", classes, new Date(2026, 8, 15, 10));
    expect(r).toMatchObject({ title: "problem set", classId: 2, estimate: 90, scheduledDate: "2026-09-15" });
  });

  it("schedules 'tomorrow' for the next local day instead of leaving it unscheduled", () => {
    const r = parseQuickCapture("LGST 1000 case brief 60m tomorrow", classes, new Date(2026, 8, 30, 10));
    expect(r.scheduledDate).toBe("2026-10-01");
    expect(r.title).toBe("case brief");
  });

  it("uses the local date late in the evening, not the UTC date", () => {
    // 11:30 pm local is already the next day in UTC for any zone west of UTC.
    const r = parseQuickCapture("CS101 read", classes, new Date(2026, 8, 15, 23, 30));
    expect(r.scheduledDate).toBe("2026-09-15");
  });

  it("lets 'today' win over 'tomorrow' and falls back to the first class", () => {
    const r = parseQuickCapture("write essay today tomorrow", classes, new Date(2026, 8, 15, 10));
    expect(r.scheduledDate).toBe("2026-09-15");
    expect(r.classId).toBe(1);
  });

  it("copes with no classes and regex characters in course codes", () => {
    expect(parseQuickCapture("something", [], new Date(2026, 8, 15)).classId).toBeNull();
    const r = parseQuickCapture("C++ lab", [{ id: 3, course_code: "C++" }], new Date(2026, 8, 15));
    expect(r.classId).toBe(3);
  });
});
