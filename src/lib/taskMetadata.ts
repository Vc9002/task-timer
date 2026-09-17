import type { TaskType } from "$lib/api";

export const TASK_TYPES: Array<{ value: TaskType; label: string }> = [
  { value: "assignment", label: "Assignment" },
  { value: "reading", label: "Reading" },
  { value: "problem_set", label: "Problem set" },
  { value: "exam", label: "Exam" },
  { value: "project", label: "Project" },
  { value: "other", label: "Other" },
];

export function taskTypeLabel(value: TaskType): string {
  return TASK_TYPES.find(type => type.value === value)?.label ?? "Assignment";
}

export function parseTags(value: string): string[] {
  return [...new Set(value.split(",").map(tag => tag.trim().toLowerCase()).filter(Boolean))].slice(0, 12);
}
