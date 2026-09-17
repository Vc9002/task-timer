use crate::db::Db;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub class_id: i64,
    pub parent_task_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_date: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub source: String,
    pub external_id: Option<String>,
    pub completed_at: Option<String>,
    pub tracked_seconds: i64,
    pub tracked_seconds_direct: i64,
    pub remaining_minutes: Option<i64>,
    pub external_state: String,
    pub task_type: String,
    pub tags: Vec<String>,
    pub time_budget_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InboxTask {
    #[serde(flatten)]
    pub task: Task,
    pub course_code: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTask {
    pub class_id: i64,
    pub parent_task_id: Option<i64>,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_date: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub task_type: Option<String>,
    pub tags: Option<Vec<String>>,
    pub time_budget_minutes: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTask {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i64>,
    pub due_at: Option<String>,
    pub scheduled_date: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub task_type: Option<String>,
    pub tags: Option<Vec<String>>,
    pub time_budget_minutes: Option<i64>,
}

// Per-row display aggregates descendants. Global analytics always sums sessions directly.
// tracked_seconds_direct counts only sessions started on this exact task, so
// remaining-time math never double-counts a parent against its subtasks.
pub(crate) const TASK_SELECT: &str = "SELECT t.*, (
    WITH RECURSIVE descendants(id) AS (
        SELECT t.id UNION SELECT child.id FROM tasks child JOIN descendants d ON child.parent_task_id=d.id
    ) SELECT COALESCE(SUM(final_duration_seconds),0) FROM time_sessions
      WHERE end_ts IS NOT NULL AND task_id IN (SELECT id FROM descendants)
) AS tracked_seconds, (
    SELECT COALESCE(SUM(final_duration_seconds),0) FROM time_sessions
      WHERE end_ts IS NOT NULL AND task_id = t.id
) AS tracked_seconds_direct FROM tasks t";

pub(crate) const ELIGIBLE_TASK_CLAUSE: &str = "t.class_id IN (SELECT id FROM classes WHERE active=1) AND
    t.is_class_timer = 0 AND
    (t.source='local' OR (t.external_state='active' AND EXISTS
        (SELECT 1 FROM todoist_projects p WHERE p.todoist_id=t.todoist_project_id AND p.class_id=t.class_id)))";

pub(crate) fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let estimated_minutes: Option<i64> = row.get("estimated_minutes")?;
    let tracked_seconds_direct: i64 = row.get("tracked_seconds_direct")?;
    let remaining_minutes = estimated_minutes.map(|e| (e - tracked_seconds_direct / 60).max(0));
    let tags_json: String = row.get("tags")?;
    let tags = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(Task {
        id: row.get("id")?,
        class_id: row.get("class_id")?,
        parent_task_id: row.get("parent_task_id")?,
        title: row.get("title")?,
        description: row.get("description")?,
        status: row.get("status")?,
        priority: row.get("priority")?,
        due_at: row.get("due_at")?,
        scheduled_date: row.get("scheduled_date")?,
        estimated_minutes,
        source: row.get("source")?,
        external_id: row.get("external_id")?,
        completed_at: row.get("completed_at")?,
        tracked_seconds: row.get("tracked_seconds")?,
        tracked_seconds_direct,
        remaining_minutes,
        external_state: row.get("external_state")?,
        task_type: row.get("task_type")?,
        tags,
        time_budget_minutes: row.get("time_budget_minutes")?,
    })
}

#[tauri::command]
pub fn list_tasks_for_class(db: State<Db>, class_id: i64) -> Result<Vec<Task>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!(
            "{TASK_SELECT} WHERE class_id = ?1 AND is_class_timer = 0 ORDER BY due_at IS NULL, due_at, id"
        ))
        .map_err(|e| e.to_string())?;
    let result = stmt
        .query_map([class_id], row_to_task)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn list_inbox(db: State<Db>) -> Result<Vec<InboxTask>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!(
            "SELECT q.*, c.course_code FROM ({TASK_SELECT}) q
             JOIN classes c ON q.class_id = c.id
             WHERE c.active=1
               AND (q.source='local' OR (q.external_state='active' AND EXISTS
                   (SELECT 1 FROM todoist_projects p WHERE p.todoist_id=q.todoist_project_id AND p.class_id=q.class_id)))
               AND q.status != 'completed' AND q.scheduled_date IS NULL
             ORDER BY q.priority IS NULL, q.priority DESC, q.due_at IS NULL, q.due_at, q.id"
        ))
        .map_err(|e| e.to_string())?;
    let result = stmt
        .query_map([], |row| {
            Ok(InboxTask {
                task: row_to_task(row)?,
                course_code: row.get("course_code")?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn create_task(db: State<Db>, input: NewTask) -> Result<Task, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    validate_new_task(&conn, &input)?;
    conn.execute(
        "INSERT INTO tasks (class_id, parent_task_id, title, description, priority,
         due_at, scheduled_date, estimated_minutes, task_type, tags, time_budget_minutes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![
            input.class_id,
            input.parent_task_id,
            input.title,
            input.description,
            input.priority,
            input.due_at,
            input.scheduled_date,
            input.estimated_minutes,
            input.task_type.as_deref().unwrap_or("assignment"),
            serde_json::to_string(&normalize_tags(input.tags.as_deref().unwrap_or(&[])))
                .map_err(|e| e.to_string())?,
            input.time_budget_minutes,
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(&format!("{TASK_SELECT} WHERE id = ?1"), [id], row_to_task)
        .map_err(|e| e.to_string())
}

fn validate_new_task(conn: &rusqlite::Connection, input: &NewTask) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("Enter a task title.".into());
    }
    if input.estimated_minutes.is_some_and(|n| n < 0) {
        return Err("Estimate must be zero or greater.".into());
    }
    validate_metadata(
        input.task_type.as_deref(),
        input.tags.as_deref(),
        input.time_budget_minutes,
    )?;
    for date in [&input.due_at, &input.scheduled_date].into_iter().flatten() {
        if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return Err("Choose a valid date.".into());
        }
    }
    if let Some(parent) = input.parent_task_id {
        let valid: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1 AND class_id=?2 AND source='local' AND status!='completed')",rusqlite::params![parent,input.class_id],|r| r.get(0)).map_err(|_| "Couldn't check parent task")?;
        if !valid {
            return Err("Choose an incomplete local parent in the same class.".into());
        }
    }
    Ok(())
}

fn validate_metadata(
    task_type: Option<&str>,
    tags: Option<&[String]>,
    budget: Option<i64>,
) -> Result<(), String> {
    if let Some(value) = task_type {
        if ![
            "assignment",
            "reading",
            "problem_set",
            "exam",
            "project",
            "other",
        ]
        .contains(&value)
        {
            return Err("Choose a valid task type.".into());
        }
    }
    if budget.is_some_and(|n| n < 0) {
        return Err("Time budget must be zero or greater.".into());
    }
    if tags.is_some_and(|items| {
        items.len() > 12
            || items
                .iter()
                .any(|tag| tag.trim().is_empty() || tag.len() > 32)
    }) {
        return Err("Use up to 12 non-empty tags of 32 characters or fewer.".into());
    }
    Ok(())
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for tag in tags
        .iter()
        .map(|tag| tag.trim().to_lowercase())
        .filter(|tag| !tag.is_empty())
    {
        if !result.contains(&tag) {
            result.push(tag);
        }
    }
    result.truncate(12);
    result
}

#[tauri::command]
pub fn update_task(
    app: tauri::AppHandle,
    db: State<Db>,
    input: UpdateTask,
) -> Result<Task, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    validate_metadata(
        input.task_type.as_deref(),
        input.tags.as_deref(),
        input.time_budget_minutes,
    )?;
    conn.execute(
        "UPDATE tasks SET title = CASE WHEN source='local' THEN ?1 ELSE title END,
         description = CASE WHEN source='local' THEN ?2 ELSE description END,
         priority = CASE WHEN source='local' THEN ?3 ELSE priority END,
         due_at = CASE WHEN source='local' THEN ?4 ELSE due_at END,
         scheduled_date = ?5, estimated_minutes = ?6,
         task_type = CASE WHEN source='local' THEN ?7 ELSE task_type END,
         tags = CASE WHEN source='local' THEN ?8 ELSE tags END,
         time_budget_minutes = ?9, updated_at = datetime('now')
         WHERE id = ?10",
        rusqlite::params![
            input.title,
            input.description,
            input.priority,
            input.due_at,
            input.scheduled_date,
            input.estimated_minutes,
            input.task_type.as_deref().unwrap_or("assignment"),
            serde_json::to_string(&normalize_tags(input.tags.as_deref().unwrap_or(&[])))
                .map_err(|e| e.to_string())?,
            input.time_budget_minutes,
            input.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    let result = conn
        .query_row(
            &format!("{TASK_SELECT} WHERE id = ?1"),
            [input.id],
            row_to_task,
        )
        .map_err(|e| e.to_string());
    drop(conn);
    crate::reminders::wake(&app);
    result
}

#[tauri::command]
pub fn duplicate_task(db: State<Db>, id: i64) -> Result<Task, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let source: String = conn
        .query_row("SELECT source FROM tasks WHERE id=?1", [id], |row| {
            row.get(0)
        })
        .map_err(|_| "Task not found")?;
    if source != "local" {
        return Err("Only local tasks can be duplicated.".into());
    }
    conn.execute(
        "INSERT INTO tasks(class_id,parent_task_id,title,description,priority,due_at,
         scheduled_date,estimated_minutes,task_type,tags,time_budget_minutes)
         SELECT class_id,parent_task_id,title || ' (copy)',description,priority,due_at,
         scheduled_date,estimated_minutes,task_type,tags,time_budget_minutes
         FROM tasks WHERE id=?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    let copy_id = conn.last_insert_rowid();
    conn.query_row(
        &format!("{TASK_SELECT} WHERE id=?1"),
        [copy_id],
        row_to_task,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_task_status(db: State<Db>, id: i64, status: String) -> Result<Task, String> {
    if !["not_started", "in_progress", "completed"].contains(&status.as_str()) {
        return Err(format!("invalid status: {status}"));
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let source: String = conn
        .query_row("SELECT source FROM tasks WHERE id=?1", [id], |r| r.get(0))
        .map_err(|_| "Task not found")?;
    if source == "todoist" {
        return Err(
            "Complete this task in Todoist, then sync. Completion is read-only here.".into(),
        );
    }
    let completed_at_clause = if status == "completed" {
        "datetime('now')"
    } else {
        "NULL"
    };
    conn.execute(
        &format!(
            "UPDATE tasks SET status = ?1, completed_at = {completed_at_clause},
             updated_at = datetime('now') WHERE id = ?2"
        ),
        rusqlite::params![status, id],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(&format!("{TASK_SELECT} WHERE id = ?1"), [id], row_to_task)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn schedule_task(db: State<Db>, id: i64, date: Option<String>) -> Result<(), String> {
    if let Some(value) = &date {
        chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|_| "Choose a valid schedule date")?;
    }
    let conn = db.0.lock().map_err(|_| "Couldn't access tasks")?;
    conn.execute(
        "UPDATE tasks SET scheduled_date=?1, updated_at=datetime('now') WHERE id=?2",
        rusqlite::params![date, id],
    )
    .map_err(|_| "Couldn't save the schedule. Try again.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_normalized_and_validated() {
        let tags = vec![" Exam ".into(), "exam".into(), "reading".into()];
        validate_metadata(Some("exam"), Some(&tags), Some(90)).unwrap();
        assert_eq!(normalize_tags(&tags), vec!["exam", "reading"]);
        assert!(validate_metadata(Some("invalid"), None, None).is_err());
        assert!(validate_metadata(None, None, Some(-1)).is_err());
    }
}
