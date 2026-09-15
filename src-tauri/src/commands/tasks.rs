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
    pub external_state: String,
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
}

// Per-row display aggregates descendants. Global analytics always sums sessions directly.
pub(crate) const TASK_SELECT: &str = "SELECT t.*, (
    WITH RECURSIVE descendants(id) AS (
        SELECT t.id UNION SELECT child.id FROM tasks child JOIN descendants d ON child.parent_task_id=d.id
    ) SELECT COALESCE(SUM(final_duration_seconds),0) FROM time_sessions
      WHERE end_ts IS NOT NULL AND task_id IN (SELECT id FROM descendants)
) AS tracked_seconds FROM tasks t";

pub(crate) fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
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
        estimated_minutes: row.get("estimated_minutes")?,
        source: row.get("source")?,
        external_id: row.get("external_id")?,
        completed_at: row.get("completed_at")?,
        tracked_seconds: row.get("tracked_seconds")?,
        external_state: row.get("external_state")?,
    })
}

#[tauri::command]
pub fn list_tasks_for_class(db: State<Db>, class_id: i64) -> Result<Vec<Task>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!(
            "{TASK_SELECT} WHERE class_id = ?1 ORDER BY due_at IS NULL, due_at, id"
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
pub fn create_task(db: State<Db>, input: NewTask) -> Result<Task, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO tasks (class_id, parent_task_id, title, description, priority,
         due_at, scheduled_date, estimated_minutes)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            input.class_id,
            input.parent_task_id,
            input.title,
            input.description,
            input.priority,
            input.due_at,
            input.scheduled_date,
            input.estimated_minutes,
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(&format!("{TASK_SELECT} WHERE id = ?1"), [id], row_to_task)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_task(db: State<Db>, input: UpdateTask) -> Result<Task, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tasks SET title = CASE WHEN source='local' THEN ?1 ELSE title END,
         description = CASE WHEN source='local' THEN ?2 ELSE description END,
         priority = CASE WHEN source='local' THEN ?3 ELSE priority END,
         due_at = CASE WHEN source='local' THEN ?4 ELSE due_at END,
         scheduled_date = ?5, estimated_minutes = ?6, updated_at = datetime('now')
         WHERE id = ?7",
        rusqlite::params![
            input.title,
            input.description,
            input.priority,
            input.due_at,
            input.scheduled_date,
            input.estimated_minutes,
            input.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        &format!("{TASK_SELECT} WHERE id = ?1"),
        [input.id],
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
