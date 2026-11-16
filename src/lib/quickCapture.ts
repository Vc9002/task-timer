import { localDate } from "./format";

export interface CaptureClass { id: number; course_code: string }

export interface ParsedCapture {
  title: string;
  classId: number | null;
  estimate: number | null;
  scheduledDate: string | null;
}

const escapeRegex = (text: string) => text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/**
 * Parses quick-capture shorthand such as "LGST case brief 60m tomorrow".
 * "today" wins over "tomorrow"; with neither word the task is scheduled today.
 * Dates are the user's local calendar days, never UTC.
 */
export function parseQuickCapture(raw: string, classes: CaptureClass[], now = new Date()): ParsedCapture {
  const lower = raw.toLowerCase();
  const matchedClass = classes.find(c => lower.includes(c.course_code.toLowerCase()));
  const durationMatch = raw.match(/(\d+)\s*m(?:in(?:ute)?s?)?/i);
  const codes = classes.map(c => escapeRegex(c.course_code));
  const removable = new RegExp(
    `\\b(?:${[...codes, "\\d+\\s*m(?:in(?:ute)?s?)?", "today", "tomorrow"].join("|")})\\b`,
    "gi",
  );
  let scheduledDate: string | null = localDate(now);
  if (!lower.includes("today") && lower.includes("tomorrow")) {
    const next = new Date(now);
    next.setDate(next.getDate() + 1);
    scheduledDate = localDate(next);
  }
  return {
    title: raw.replace(removable, "").replace(/\s+/g, " ").trim(),
    classId: matchedClass?.id ?? classes[0]?.id ?? null,
    estimate: durationMatch ? Number(durationMatch[1]) : null,
    scheduledDate,
  };
}
