import { invoke } from "@tauri-apps/api/core";

export function exportData(format: "csv" | "tasks" | "json" | "ics"): Promise<boolean> { return invoke("export_data", { format }); }
export interface NotificationPreferences { enabled: boolean; overrun_percent: number }
export function notificationSettings(): Promise<NotificationPreferences> { return invoke("notification_settings"); }
export function saveNotificationSettings(settings: NotificationPreferences): Promise<void> { return invoke("save_notification_settings", { settings }); }
export function testNotification(): Promise<void> { return invoke("test_notification"); }

export function getIdleThreshold(): Promise<number> { return invoke("get_idle_threshold"); }
export function saveIdleThreshold(minutes: number): Promise<void> { return invoke("save_idle_threshold", { minutes }); }

export interface PomodoroSettings { work_minutes: number; break_minutes: number; long_break_minutes: number }
export function getPomodoroSettings(): Promise<PomodoroSettings> { return invoke("get_pomodoro_settings"); }
export function savePomodoroSettings(settings: PomodoroSettings): Promise<void> { return invoke("save_pomodoro_settings", { settings }); }
export function notifyPomodoroPhase(phase: "work_done" | "break_done"): Promise<void> { return invoke("notify_pomodoro_phase", { phase }); }
export function setTrayPomodoroStatus(text: string | null): Promise<void> { return invoke("set_tray_pomodoro_status", { text }); }

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
  notes: string | null;
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
  task_type: TaskType;
  tags: string[];
  time_budget_minutes: number | null;
  scheduled_minutes_before_due: number;
  unplanned_minutes: number;
  schedule_coverage_percent: number;
}

export type TaskType = "assignment" | "reading" | "problem_set" | "exam" | "project" | "other";

export interface NewTask {
  class_id: number;
  parent_task_id: number | null;
  title: string;
  description: string | null;
  priority: number | null;
  due_at: string | null;
  scheduled_date: string | null;
  estimated_minutes: number | null;
  task_type?: TaskType;
  tags?: string[];
  time_budget_minutes?: number | null;
}

export function listTasksForClass(classId: number): Promise<TaskRecord[]> {
  return invoke("list_tasks_for_class", { classId });
}

export interface InboxTask extends TaskRecord { course_code: string; }

export function listInbox(): Promise<InboxTask[]> {
  return invoke("list_inbox");
}

export function createTask(input: NewTask): Promise<TaskRecord> {
  return invoke("create_task", { input });
}

export interface UpdateTask {
  id: number;
  title: string;
  description: string | null;
  notes: string | null;
  priority: number | null;
  due_at: string | null;
  scheduled_date: string | null;
  estimated_minutes: number | null;
  task_type?: TaskType;
  tags?: string[];
  time_budget_minutes?: number | null;
}

export function updateTask(input: UpdateTask): Promise<TaskRecord> {
  return invoke("update_task", { input });
}

export function duplicateTask(id: number): Promise<TaskRecord> {
  return invoke("duplicate_task", { id });
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

export interface StudyBlock {
  id: number; task_id: number; task_title: string; class_id: number; course_code: string;
  planned_date: string; planned_start_time: string | null; planned_minutes: number; completed: boolean;
}
export interface NewStudyBlock { task_id: number; planned_date: string; planned_start_time: string | null; planned_minutes: number; }
export interface UpdateStudyBlock extends Omit<NewStudyBlock, "task_id"> { id: number; completed: boolean; }
export function createStudyBlock(input: NewStudyBlock): Promise<StudyBlock> { return invoke("create_study_block", { input }); }
export function updateStudyBlock(input: UpdateStudyBlock): Promise<StudyBlock> { return invoke("update_study_block", { input }); }
export function deleteStudyBlock(id: number): Promise<void> { return invoke("delete_study_block", { id }); }
export function listStudyBlocksForTask(taskId: number): Promise<StudyBlock[]> { return invoke("list_study_blocks_for_task", { taskId }); }
export function getStudyBlocksForRange(startDate: string, endDate: string): Promise<StudyBlock[]> { return invoke("get_study_blocks_for_range", { startDate, endDate }); }

export interface TaskMilestone { id: number; task_id: number; title: string; target_date: string | null; position: number; completed: boolean; completed_at: string | null; }
export interface NewMilestone { task_id: number; title: string; target_date: string | null; }
export interface UpdateMilestone { id: number; title: string; target_date: string | null; completed: boolean; }
export function listTaskMilestones(taskId: number): Promise<TaskMilestone[]> { return invoke("list_task_milestones", { taskId }); }
export function createTaskMilestone(input: NewMilestone): Promise<TaskMilestone> { return invoke("create_task_milestone", { input }); }
export function updateTaskMilestone(input: UpdateMilestone): Promise<TaskMilestone> { return invoke("update_task_milestone", { input }); }
export function deleteTaskMilestone(id: number): Promise<void> { return invoke("delete_task_milestone", { id }); }
export function moveTaskMilestone(id: number, direction: "up" | "down"): Promise<void> { return invoke("move_task_milestone", { id, direction }); }

export interface TaskTemplate { id: number; name: string; class_id: number | null; task_type: TaskType | null; default_estimated_minutes: number | null; default_time_budget_minutes: number | null; priority: number | null; tags: string[]; }
export interface NewTaskTemplate { name: string; class_id: number | null; task_type: TaskType | null; default_estimated_minutes: number | null; default_time_budget_minutes: number | null; priority: number | null; tags: string[]; }
export interface InstantiateTemplate { template_id: number; class_id: number; title: string; due_at: string | null; scheduled_date: string | null; }
export function listTaskTemplates(): Promise<TaskTemplate[]> { return invoke("list_task_templates"); }
export function createTaskTemplate(input: NewTaskTemplate): Promise<TaskTemplate> { return invoke("create_task_template", { input }); }
export function deleteTaskTemplate(id: number): Promise<void> { return invoke("delete_task_template", { id }); }
export function instantiateTaskTemplate(input: InstantiateTemplate): Promise<TaskRecord> { return invoke("instantiate_task_template", { input }); }
export interface TemplateMilestone { id: number; template_id: number; title: string; offset_days_before_due: number | null; position: number; }
export function listTemplateMilestones(templateId: number): Promise<TemplateMilestone[]> { return invoke("list_template_milestones", { templateId }); }
export function createTemplateMilestone(input: { template_id: number; title: string; offset_days_before_due: number | null }): Promise<TemplateMilestone> { return invoke("create_template_milestone", { input }); }
export function deleteTemplateMilestone(id: number): Promise<void> { return invoke("delete_template_milestone", { id }); }

export interface Exam { id: number; class_id: number; course_code: string; title: string; exam_date: string; target_study_minutes: number | null; tracked_minutes: number; study_task_id: number | null; }
export function listExams(): Promise<Exam[]> { return invoke("list_exams"); }
export function createExam(input: { class_id: number; title: string; exam_date: string; target_study_minutes: number | null }): Promise<Exam> { return invoke("create_exam", { input }); }
export function deleteExam(id: number): Promise<void> { return invoke("delete_exam", { id }); }
export interface SemesterClassSummary { class_id: number; course_code: string; tracked_seconds: number; completed_tasks: number; open_tasks: number; upcoming_deadlines: number; }
export interface SemesterDashboard { semester: string; tracked_seconds: number; completed_tasks: number; upcoming_deadlines: number; classes: SemesterClassSummary[]; }
export function getSemesterDashboard(semester: string): Promise<SemesterDashboard> { return invoke("get_semester_dashboard", { semester }); }

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
  context_only?: boolean;
  parent_path?: string | null;
  scheduled_minutes_before_due: number;
  unplanned_minutes: number;
  schedule_coverage_percent: number;
}

export interface WeekStudyBlock extends StudyBlock {}

export interface WeekDay {
  date: string;
  tasks: WeekTask[];
  estimated_minutes_remaining: number;
  tracked_seconds: number;
  capacity_minutes: number | null;
  load_percent: number | null;
  study_blocks: WeekStudyBlock[];
  study_block_minutes: number;
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
export interface UpdateRecurringTemplate extends NewRecurringTemplate { id: number; }
export function updateRecurringTemplate(input: UpdateRecurringTemplate): Promise<RecurringTemplate> { return invoke("update_recurring_template", { input }); }
export interface CalendarDay { date: string; planned: TaskRecord[]; due: TaskRecord[]; study_blocks: StudyBlock[]; }
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

export function startClassTimer(classId: number): Promise<ActiveSessionInfo> {
  return invoke("start_class_timer", { classId });
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

export interface EstimateBreakdown {
  key: string;
  label: string;
  sample_count: number;
  median_estimated_minutes: number;
  median_actual_minutes: number;
  median_error_percent: number;
}
export interface EstimateBand { label: string; count: number; }
export interface SessionStats {
  count: number;
  median_seconds: number;
  average_seconds: number;
  longest_seconds: number;
  under_ten_minutes: number;
  at_least_thirty_minutes: number;
}
export interface BlockActualSummary {
  planned_minutes: number;
  completed_planned_minutes: number;
  actual_tracked_seconds: number;
  block_count: number;
  completed_block_count: number;
}
export interface EstimateAnalytics {
  sample_count: number;
  median_estimated_minutes: number;
  median_actual_minutes: number;
  median_difference_minutes: number;
  median_error_percent: number;
  bands: EstimateBand[];
  by_class: EstimateBreakdown[];
  by_task_type: EstimateBreakdown[];
  session_stats: SessionStats;
  blocks: BlockActualSummary;
}
export interface EstimateSuggestion {
  suggested_minutes: number | null;
  sample_count: number;
  source: string | null;
  median_actual_minutes: number | null;
}
export interface EstimateMiss {
  task_id: number;
  task_title: string;
  course_code: string;
  estimated_minutes: number;
  actual_minutes: number;
  difference_minutes: number;
  error_percent: number;
}
export interface WorkloadDay {
  date: string;
  available_minutes: number | null;
  scheduled_block_minutes: number;
  remaining_due_minutes: number;
  remaining_capacity_minutes: number | null;
  overflow_minutes: number;
  load_percent: number | null;
}
export interface WeeklyReview {
  start_date: string;
  end_date: string;
  tracked_seconds: number;
  completed_tasks: number;
  overdue_tasks: number;
  largest_estimate_miss: EstimateMiss | null;
  most_time_class: ClassTotal | null;
  median_session_seconds: number;
  schedule_coverage_percent: number;
  blocks: BlockActualSummary;
  workload: WorkloadDay[];
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

export function getEstimateAnalytics(startDate: string, endDate: string): Promise<EstimateAnalytics> {
  return invoke("get_estimate_analytics", { startDate, endDate });
}
export function getEstimateSuggestion(classId: number, taskType: TaskType): Promise<EstimateSuggestion> {
  return invoke("get_estimate_suggestion", { classId, taskType });
}
export function getWeeklyReview(startDate: string): Promise<WeeklyReview> {
  return invoke("get_weekly_review", { startDate });
}

export interface ProposedBlock { task_id: number; task_title: string; course_code: string; planned_date: string; planned_minutes: number; reason: string; }
export interface PlanDay { date: string; capacity_minutes: number | null; existing_block_minutes: number; proposed_minutes: number; blocks: ProposedBlock[]; }
export interface PlanProposal { start_date: string; end_date: string; days: PlanDay[]; unschedulable_overdue: string[]; }
export function getPlanProposal(startDate: string, days: number): Promise<PlanProposal> { return invoke("get_plan_proposal", { input: { startDate, days } }); }
export function applyPlan(blocks: Array<{ task_id: number; planned_date: string; planned_minutes: number }>): Promise<number> { return invoke("apply_plan", { input: { blocks } }); }

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
export interface TodoistOutboxEntry { id: number; external_id: string; status: "pending" | "sending" | "completed" | "failed"; attempts: number; last_error: string | null; updated_at: string; }
export function getTodoistOutboxStatus(): Promise<TodoistOutboxEntry[]> { return invoke("get_todoist_outbox_status"); }

export function switchTimer(sessionId: number, taskId: number): Promise<ActiveSessionInfo> {
  return invoke("switch_timer", { sessionId, taskId });
}
export function recoverTimer(sessionId: number, durationSeconds: number): Promise<Session> {
  return invoke("recover_timer", { sessionId, durationSeconds });
}

export function scheduleTask(id: number, date: string | null): Promise<void> {
  return invoke("schedule_task", { id, date });
}
