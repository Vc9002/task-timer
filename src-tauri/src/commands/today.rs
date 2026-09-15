use crate::db::Db;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct TodayTask {
    pub id: i64,
    pub parent_task_id: Option<i64>,
    pub title: String,
    pub status: String,
    pub estimated_minutes: Option<i64>,
    pub due_at: Option<String>,
    pub overdue: bool,
    pub source: String,
    /// Tracked seconds for this task plus all descendant subtasks, no double counting.
    pub tracked_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct TodayClassGroup {
    pub class_id: i64,
    pub course_code: String,
    pub tasks: Vec<TodayTask>,
}

#[derive(Debug, Serialize)]
pub struct TodaySummary {
    pub groups: Vec<TodayClassGroup>,
    pub task_count: i64,
    pub estimated_minutes_total: i64,
    pub tracked_seconds_total: i64,
}

/// A task is "on Today" if: due today, overdue & incomplete (when enabled),
/// or explicitly scheduled for today.
const TODAY_TASK_IDS_SQL: &str = "
    SELECT DISTINCT t.id FROM tasks t
    WHERE t.status != 'completed' AND (t.source='local' OR (t.external_state='active' AND EXISTS (SELECT 1 FROM todoist_projects p WHERE p.todoist_id=t.todoist_project_id AND p.class_id=t.class_id))) AND (
        date(t.due_at) = date('now', 'localtime')
        OR (?1 = 1 AND t.due_at IS NOT NULL AND date(t.due_at) < date('now', 'localtime'))
        OR t.scheduled_date = date('now', 'localtime')
    )
";

/// Recursive CTE: sum of finished session durations for a task and all its descendants.
const TRACKED_SECONDS_SQL: &str = "
    WITH RECURSIVE descendants(id) AS (
        SELECT ?1
        UNION ALL
        SELECT tasks.id FROM tasks JOIN descendants ON tasks.parent_task_id = descendants.id
    )
    SELECT COALESCE(SUM(
        COALESCE(ts.final_duration_seconds,
            CASE WHEN ts.end_ts IS NOT NULL
                THEN CAST((julianday(ts.end_ts) - julianday(ts.start_ts)) * 86400 AS INTEGER)
                     - ts.accumulated_pause_seconds
                ELSE 0 END)
    ), 0)
    FROM time_sessions ts WHERE ts.task_id IN (SELECT id FROM descendants)
";

#[tauri::command]
pub fn get_today(db: State<Db>, include_overdue: bool) -> Result<TodaySummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let mut id_stmt = conn
        .prepare(TODAY_TASK_IDS_SQL)
        .map_err(|e| e.to_string())?;
    let task_ids: Vec<i64> = id_stmt
        .query_map([include_overdue as i64], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut groups: std::collections::BTreeMap<i64, TodayClassGroup> =
        std::collections::BTreeMap::new();
    let mut task_count = 0i64;
    let mut estimated_minutes_total = 0i64;
    let mut tracked_seconds_total = 0i64;

    let mut task_stmt = conn
        .prepare("SELECT * FROM tasks WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut class_stmt = conn
        .prepare("SELECT course_code FROM classes WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut tracked_stmt = conn
        .prepare(TRACKED_SECONDS_SQL)
        .map_err(|e| e.to_string())?;

    for task_id in task_ids {
        let row = task_stmt
            .query_row([task_id], |row| {
                Ok((
                    row.get::<_, i64>("class_id")?,
                    row.get::<_, Option<i64>>("parent_task_id")?,
                    row.get::<_, String>("title")?,
                    row.get::<_, String>("status")?,
                    row.get::<_, Option<i64>>("estimated_minutes")?,
                    row.get::<_, Option<String>>("due_at")?,
                    row.get::<_, String>("source")?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let (class_id, parent_task_id, title, status, estimated_minutes, due_at, source) = row;

        let today_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        let overdue = due_at
            .as_deref()
            .map(|d| d < today_str.as_str())
            .unwrap_or(false);

        let tracked_seconds: i64 = tracked_stmt
            .query_row([task_id], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        task_count += 1;
        estimated_minutes_total += estimated_minutes.unwrap_or(0);
        // Only count top-level tracked time once toward the grand total to avoid
        // double counting a parent's aggregate alongside its own subtask entries.
        if parent_task_id.is_none() {
            tracked_seconds_total += tracked_seconds;
        }

        let course_code: String = class_stmt
            .query_row([class_id], |r| r.get(0))
            .map_err(|e| e.to_string())?;

        let group = groups.entry(class_id).or_insert_with(|| TodayClassGroup {
            class_id,
            course_code,
            tasks: Vec::new(),
        });

        group.tasks.push(TodayTask {
            id: task_id,
            parent_task_id,
            title,
            status,
            estimated_minutes,
            due_at,
            overdue,
            source,
            tracked_seconds,
        });
    }

    Ok(TodaySummary {
        groups: groups.into_values().collect(),
        task_count,
        estimated_minutes_total,
        tracked_seconds_total,
    })
}
