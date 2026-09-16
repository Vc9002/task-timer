export const PRIORITY_LABELS: Record<number, string> = {
  1: "Low",
  2: "Normal",
  3: "High",
  4: "Urgent",
};

export const DEFAULT_PRIORITY = 2;

export function priorityLabel(priority: number | null): string {
  return PRIORITY_LABELS[priority ?? DEFAULT_PRIORITY] ?? "Normal";
}

// Todoist's priority field already runs 1 (lowest) to 4 (urgent), matching
// our scale numerically — but Todoist tasks with no priority set arrive as
// null, not 1, so that case must fall through to our own default rather
// than being assumed equal to Todoist's.
export function fromTodoistPriority(todoistPriority: number | null): number {
  if (todoistPriority === null) return DEFAULT_PRIORITY;
  return Math.min(4, Math.max(1, todoistPriority));
}
