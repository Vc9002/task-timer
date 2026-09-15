import { invoke } from "@tauri-apps/api/core";

export interface ClassRecord {
  id: number;
  course_code: string;
  name: string | null;
  semester: string;
  color: string | null;
  active: boolean;
  todoist_project_id: string | null;
}

export interface NewClass {
  course_code: string;
  name: string | null;
  semester: string;
  color: string | null;
}

export interface UpdateClass {
  id: number;
  course_code: string;
  name: string | null;
  semester: string;
  color: string | null;
  active: boolean;
}

export function listClasses(includeInactive = false): Promise<ClassRecord[]> {
  return invoke("list_classes", { includeInactive });
}

export function createClass(input: NewClass): Promise<ClassRecord> {
  return invoke("create_class", { input });
}

export function updateClass(input: UpdateClass): Promise<ClassRecord> {
  return invoke("update_class", { input });
}

export function archiveClass(id: number): Promise<void> {
  return invoke("archive_class", { id });
}
