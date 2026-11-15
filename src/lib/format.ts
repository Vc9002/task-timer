export function formatHms(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return h > 0 ? `${pad(h)}:${pad(m)}:${pad(sec)}` : `${pad(m)}:${pad(sec)}`;
}

export function formatDurationShort(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function formatMinutesShort(totalMinutes: number): string {
  const m = Math.max(0, Math.round(totalMinutes));
  const h = Math.floor(m / 60);
  const rem = m % 60;
  if (h > 0) return rem > 0 ? `${h}h ${rem}m` : `${h}h`;
  return `${rem}m`;
}

export function formatSignedDurationShort(deltaSeconds: number): string {
  const sign = deltaSeconds >= 0 ? "+" : "-";
  return `${sign}${formatDurationShort(Math.abs(deltaSeconds))}`;
}

export function localDate(date = new Date()): string {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
}

/**
 * Short label for a due date such as "Nov 15". Date-only values are floating
 * local days, so they are never passed through `new Date(string)`, which would
 * read them as UTC and show the previous day west of Greenwich.
 */
export function formatDue(dueAt: string, locale?: string): string {
  const dateOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dueAt);
  const date = dateOnly
    ? new Date(Number(dateOnly[1]), Number(dateOnly[2]) - 1, Number(dateOnly[3]))
    : new Date(dueAt.includes("T") ? dueAt : dueAt.replace(" ", "T") + "Z");
  if (Number.isNaN(date.getTime())) return dueAt.slice(0, 10);
  return date.toLocaleDateString(locale, { month: "short", day: "numeric" });
}
