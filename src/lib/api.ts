import { invoke } from "@tauri-apps/api/core";

export function exportData(format: "csv" | "tasks" | "json"): Promise<boolean> { return invoke("export_data", { format }); }
export interface NotificationPreferences { enabled: boolean; overrun_percent: number }
export function notificationSettings(): Promise<NotificationPreferences> { return invoke("notification_settings"); }
export function saveNotificationSettings(settings: NotificationPreferences): Promise<void> { return invoke("save_notification_settings", { settings }); }
export function testNotification(): Promise<void> { return invoke("test_notification"); }

// ---------- Classes ----------

export interface ClassRecord {
  id: number;
  course_code: string;
  name: string | null;
  semester: string;
  color: string | null;
  active: boolean;
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

// ---------- Tasks ----------

export interface TaskRecord {
  id: number;
  class_id: number;
  parent_task_id: number | null;
  title: string;
  description: string | null;
  status: "not_started" | "in_progress" | "completed";
  priority: number | null;
  due_at: string | null;
  scheduled_date: string | null;
  estimated_minutes: number | null;
  source: "local" | "todoist";
  external_id: string | null;
  completed_at: string | null;
  tracked_seconds: number;
  tracked_seconds_direct: number;
  remaining_minutes: number | null;
  external_state: "active" | "completed" | "deleted" | "unknown";
}

export interface NewTask {
  class_id: number;
  parent_task_id: number | null;
  title: string;
  description: string | null;
  priority: number | null;
  due_at: string | null;
  scheduled_date: string | null;
  estimated_minutes: number | null;
}

export function listTasksForClass(classId: number): Promise<TaskRecord[]> {
  return invoke("list_tasks_for_class", { classId });
}

export function createTask(input: NewTask): Promise<TaskRecord> {
  return invoke("create_task", { input });
}

export interface UpdateTask {
  id: number;
  title: string;
  description: string | null;
  priority: number | null;
  due_at: string | null;
  scheduled_date: string | null;
  estimated_minutes: number | null;
}

export function updateTask(input: UpdateTask): Promise<TaskRecord> {
  return invoke("update_task", { input });
}

export function setTaskStatus(
  id: number,
  status: TaskRecord["status"]
): Promise<TaskRecord> {
  return invoke("set_task_status", { id, status });
}

export function deleteTask(id: number): Promise<void> {
  return invoke("delete_task", { id });
}

// ---------- Today ----------

export interface TodayTask extends TaskRecord {
  overdue: boolean;
  context_only: boolean;
}

export interface TodayClassGroup {
  class_id: number;
  course_code: string;
  tasks: TodayTask[];
}

export interface TodaySummary {
  groups: TodayClassGroup[];
  task_count: number;
  estimated_minutes_total: number;
  tracked_seconds_total: number;
  next_up: TodayTask[];
}

export function getToday(includeOverdue = true): Promise<TodaySummary> {
  return invoke("get_today", { includeOverdue });
}

// ---------- Week ----------

export interface WeekTask {
  id: number;
  class_id: number;
  course_code: string;
  title: string;
  status: TaskRecord["status"];
  due_at: string | null;
  scheduled_date: string | null;
  estimated_minutes: number | null;
  tracked_seconds_direct: number;
  tracked_seconds_total: number;
  remaining_minutes: number | null;
  overdue: boolean;
}

export interface WeekDay {
  date: string;
  tasks: WeekTask[];
  estimated_minutes_remaining: number;
  tracked_seconds: number;
  capacity_minutes: number | null;
  load_percent: number | null;
}

export interface WeekSummary {
  start_date: string;
  end_date: string;
  days: WeekDay[];
  unscheduled: WeekTask[];
  estimated_minutes_remaining: number;
  tracked_seconds: number;
}

export function getWeek(startDate: string): Promise<WeekSummary> {
  return invoke("get_week", { startDate });
}

export interface CapacitySettings {
  weekday_minutes: (number | null)[];
}

export function getStudyCapacity(): Promise<CapacitySettings> {
  return invoke("get_study_capacity");
}

export function setStudyCapacity(weekdayMinutes: (number | null)[]): Promise<void> {
  return invoke("set_study_capacity", { weekdayMinutes });
}

export interface RecurringTemplate {
  id: number; class_id: number; title: string; description: string | null; priority: number;
  estimated_minutes: number | null; recurrence_type: "daily" | "weekly" | "weekdays";
  interval: number; weekdays: string | null; start_date: string; end_date: string | null; active: boolean;
}
export interface NewRecurringTemplate {
  class_id: number; title: string; description: string | null; priority: number | null;
  estimated_minutes: number | null; recurrence_type: RecurringTemplate["recurrence_type"];
  interval: number | null; weekdays: string | null; start_date: string; end_date: string | null;
}
export function listRecurringTemplates(): Promise<RecurringTemplate[]> { return invoke("list_recurring_templates"); }
export function createRecurringTemplate(input: NewRecurringTemplate): Promise<RecurringTemplate> { return invoke("create_recurring_template", { input }); }
export function setRecurringTemplateActive(id: number, active: boolean): Promise<void> { return invoke("set_recurring_template_active", { id, active }); }
export interface CalendarDay { date: string; planned: TaskRecord[]; due: TaskRecord[]; }
export function getCalendar(month: string): Promise<CalendarDay[]> { return invoke("get_calendar", { month }); }

// ---------- Timer ----------

export interface Session {
  id: number;
  task_id: number;
  start_ts: string;
  end_ts: string | null;
  accumulated_pause_seconds: number;
  pause_started_ts: string | null;
  final_duration_seconds: number | null;
  edited: boolean;
}

export interface ActiveSessionInfo {
  session: Session;
  task_title: string;
  class_course_code: string;
  is_paused: boolean;
  elapsed_seconds: number;
}

export type TimerError =
  | { kind: "ActiveSessionConflict"; task_id: number; task_title: string; class_course_code: string }
  | { kind: "NotFound" }
  | { kind: "Other"; message: string };

export function getActiveSession(): Promise<ActiveSessionInfo | null> {
  return invoke("get_active_session");
}

export function startTimer(taskId: number): Promise<ActiveSessionInfo> {
  return invoke("start_timer", { taskId });
}

export function pauseTimer(): Promise<ActiveSessionInfo> {
  return invoke("pause_timer");
}

export function resumeTimer(): Promise<ActiveSessionInfo> {
  return invoke("resume_timer");
}

export function finishTimer(): Promise<Session> {
  return invoke("finish_timer");
}

export function cancelTimer(): Promise<void> {
  return invoke("cancel_timer");
}

export function listSessionsForTask(taskId: number): Promise<Session[]> {
  return invoke("list_sessions_for_task", { taskId });
}

export function editSessionDuration(
  sessionId: number,
  finalDurationSeconds: number
): Promise<Session> {
  return invoke("edit_session_duration", {
    sessionId,
    finalDurationSeconds,
  });
}

// ---------- Analytics ----------

export interface ClassTotal {
  class_id: number;
  course_code: string;
  tracked_seconds: number;
}

export interface RangeSummary {
  tracked_seconds_total: number;
  by_class: ClassTotal[];
}

export interface SessionEntry {
  session_id: number;
  task_id: number;
  task_title: string;
  course_code: string;
  start_ts: string;
  end_ts: string | null;
  duration_seconds: number;
}

export interface DayView {
  date: string;
  total_seconds: number;
  by_class: ClassTotal[];
  sessions: SessionEntry[];
}

export interface TaskHistory {
  task_id: number;
  total_seconds: number;
  estimated_minutes: number | null;
  sessions: SessionEntry[];
}

export function getAnalyticsToday(): Promise<RangeSummary> {
  return invoke("get_analytics_today");
}

export function getAnalyticsWeek(): Promise<RangeSummary> {
  return invoke("get_analytics_week");
}

export function getAnalyticsMonth(): Promise<RangeSummary> {
  return invoke("get_analytics_month");
}

export function getDayView(date: string): Promise<DayView> {
  return invoke("get_day_view", { date });
}

export function getTaskHistory(taskId: number): Promise<TaskHistory> {
  return invoke("get_task_history", { taskId });
}

// ---------- Todoist ----------

export interface TodoistStatus {
  connected: boolean;
  last_synced_at: string | null;
}

export interface TodoistProjectMapping {
  todoist_id: string;
  name: string;
  class_id: number | null;
  synced_at: string | null;
}

export interface SyncResult {
  projects_synced: number;
  tasks_synced: number;
}

export function getTodoistStatus(): Promise<TodoistStatus> {
  return invoke("get_todoist_status");
}

export function setTodoistToken(token: string): Promise<void> {
  return invoke("set_todoist_token", { token });
}

export function disconnectTodoist(): Promise<void> {
  return invoke("disconnect_todoist");
}

export function listTodoistProjectMappings(): Promise<TodoistProjectMapping[]> {
  return invoke("list_todoist_project_mappings");
}

export function mapTodoistProject(todoistId: string, classId: number | null): Promise<void> {
  return invoke("map_todoist_project", { todoistId, classId });
}

export function syncTodoistNow(): Promise<SyncResult> {
  return invoke("sync_todoist_now");
}
export function completeTodoistTask(taskId: number): Promise<void> {
  return invoke("complete_todoist_task", { taskId });
}

export function switchTimer(sessionId: number, taskId: number): Promise<ActiveSessionInfo> {
  return invoke("switch_timer", { sessionId, taskId });
}
export function recoverTimer(sessionId: number, durationSeconds: number): Promise<Session> {
  return invoke("recover_timer", { sessionId, durationSeconds });
}

export function scheduleTask(id: number, date: string | null): Promise<void> {
  return invoke("schedule_task", { id, date });
}
