use super::tasks::{row_to_task, Task, TASK_SELECT};
use crate::db::Db;
use chrono::{Duration, NaiveDate, NaiveTime};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

fn date(value: &str) -> Result<(), String> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| "Choose a valid date.".into())
}

fn time(value: Option<&str>) -> Result<(), String> {
    if let Some(value) = value {
        NaiveTime::parse_from_str(value, "%H:%M")
            .map(|_| ())
            .map_err(|_| String::from("Choose a valid start time (HH:MM)."))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct StudyBlock {
    pub id: i64,
    pub task_id: i64,
    pub task_title: String,
    pub class_id: i64,
    pub course_code: String,
    pub planned_date: String,
    pub planned_start_time: Option<String>,
    pub planned_minutes: i64,
    pub completed: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewStudyBlock {
    pub task_id: i64,
    pub planned_date: String,
    pub planned_start_time: Option<String>,
    pub planned_minutes: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStudyBlock {
    pub id: i64,
    pub planned_date: String,
    pub planned_start_time: Option<String>,
    pub planned_minutes: i64,
    pub completed: bool,
}

fn block_row(row: &rusqlite::Row) -> rusqlite::Result<StudyBlock> {
    Ok(StudyBlock {
        id: row.get("id")?,
        task_id: row.get("task_id")?,
        task_title: row.get("task_title")?,
        class_id: row.get("class_id")?,
        course_code: row.get("course_code")?,
        planned_date: row.get("planned_date")?,
        planned_start_time: row.get("planned_start_time")?,
        planned_minutes: row.get("planned_minutes")?,
        completed: row.get::<_, i64>("completed")? != 0,
    })
}

const BLOCK_SELECT: &str = "SELECT sb.*, t.title AS task_title, t.class_id, c.course_code
    FROM study_blocks sb JOIN tasks t ON t.id=sb.task_id JOIN classes c ON c.id=t.class_id";

fn get_block(conn: &Connection, id: i64) -> Result<StudyBlock, String> {
    conn.query_row(&format!("{BLOCK_SELECT} WHERE sb.id=?1"), [id], block_row)
        .map_err(|e| e.to_string())
}

pub(crate) fn blocks_for_range(
    conn: &Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<Vec<StudyBlock>> {
    conn.prepare(&format!("{BLOCK_SELECT} WHERE sb.planned_date>=?1 AND sb.planned_date<=?2 ORDER BY sb.planned_date,sb.planned_start_time,sb.id"))
        ?.query_map(params![start_date, end_date], block_row)?
        .collect()
}

#[tauri::command]
pub fn create_study_block(db: State<Db>, input: NewStudyBlock) -> Result<StudyBlock, String> {
    date(&input.planned_date)?;
    time(input.planned_start_time.as_deref())?;
    if input.planned_minutes <= 0 {
        return Err("Study blocks must be longer than zero minutes.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let valid: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1 AND is_class_timer=0)",
            [input.task_id],
            |r| r.get(0),
        )
        .map_err(|_| "Task not found.")?;
    if !valid {
        return Err("Task not found.".into());
    }
    conn.execute(
        "INSERT INTO study_blocks(task_id,planned_date,planned_start_time,planned_minutes)
         VALUES(?1,?2,?3,?4)",
        params![
            input.task_id,
            input.planned_date,
            input.planned_start_time,
            input.planned_minutes
        ],
    )
    .map_err(|e| e.to_string())?;
    get_block(&conn, conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_study_block(db: State<Db>, input: UpdateStudyBlock) -> Result<StudyBlock, String> {
    date(&input.planned_date)?;
    time(input.planned_start_time.as_deref())?;
    if input.planned_minutes <= 0 {
        return Err("Study blocks must be longer than zero minutes.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE study_blocks SET planned_date=?1, planned_start_time=?2, planned_minutes=?3,
         completed=?4, updated_at=datetime('now') WHERE id=?5",
        params![
            input.planned_date,
            input.planned_start_time,
            input.planned_minutes,
            input.completed as i64,
            input.id
        ],
    )
    .map_err(|e| e.to_string())?;
    get_block(&conn, input.id)
}

#[tauri::command]
pub fn delete_study_block(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM study_blocks WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_study_blocks_for_task(db: State<Db>, task_id: i64) -> Result<Vec<StudyBlock>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let result = conn
        .prepare(&format!(
        "{BLOCK_SELECT} WHERE sb.task_id=?1 ORDER BY sb.planned_date,sb.planned_start_time,sb.id"
    ))
        .map_err(|e| e.to_string())?
        .query_map([task_id], block_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn get_study_blocks_for_range(
    db: State<Db>,
    start_date: String,
    end_date: String,
) -> Result<Vec<StudyBlock>, String> {
    date(&start_date)?;
    date(&end_date)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    blocks_for_range(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskMilestone {
    pub id: i64,
    pub task_id: i64,
    pub title: String,
    pub target_date: Option<String>,
    pub position: i64,
    pub completed: bool,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewMilestone {
    pub task_id: i64,
    pub title: String,
    pub target_date: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMilestone {
    pub id: i64,
    pub title: String,
    pub target_date: Option<String>,
    pub completed: bool,
}

fn milestone_row(row: &rusqlite::Row) -> rusqlite::Result<TaskMilestone> {
    Ok(TaskMilestone {
        id: row.get("id")?,
        task_id: row.get("task_id")?,
        title: row.get("title")?,
        target_date: row.get("target_date")?,
        position: row.get("position")?,
        completed: row.get::<_, i64>("completed")? != 0,
        completed_at: row.get("completed_at")?,
    })
}

fn get_milestone(conn: &Connection, id: i64) -> Result<TaskMilestone, String> {
    conn.query_row(
        "SELECT * FROM task_milestones WHERE id=?1",
        [id],
        milestone_row,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_task_milestones(db: State<Db>, task_id: i64) -> Result<Vec<TaskMilestone>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let result = conn
        .prepare("SELECT * FROM task_milestones WHERE task_id=?1 ORDER BY position,id")
        .map_err(|e| e.to_string())?
        .query_map([task_id], milestone_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn create_task_milestone(db: State<Db>, input: NewMilestone) -> Result<TaskMilestone, String> {
    if input.title.trim().is_empty() {
        return Err("Milestone title cannot be empty.".into());
    }
    if let Some(value) = &input.target_date {
        date(value)?;
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let position: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(position)+1,0) FROM task_milestones WHERE task_id=?1",
            [input.task_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO task_milestones(task_id,title,target_date,position) VALUES(?1,?2,?3,?4)",
        params![
            input.task_id,
            input.title.trim(),
            input.target_date,
            position
        ],
    )
    .map_err(|e| e.to_string())?;
    get_milestone(&conn, conn.last_insert_rowid())
}

#[tauri::command]
pub fn update_task_milestone(
    db: State<Db>,
    input: UpdateMilestone,
) -> Result<TaskMilestone, String> {
    if input.title.trim().is_empty() {
        return Err("Milestone title cannot be empty.".into());
    }
    if let Some(value) = &input.target_date {
        date(value)?;
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE task_milestones SET title=?1,target_date=?2,completed=?3,completed_at=CASE WHEN ?3=1 THEN datetime('now') ELSE NULL END,updated_at=datetime('now') WHERE id=?4", params![input.title.trim(),input.target_date,input.completed as i64,input.id]).map_err(|e| e.to_string())?;
    get_milestone(&conn, input.id)
}

#[tauri::command]
pub fn delete_task_milestone(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM task_milestones WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn move_task_milestone(db: State<Db>, id: i64, direction: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let (task_id, position): (i64, i64) = conn
        .query_row(
            "SELECT task_id,position FROM task_milestones WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Milestone not found")?;
    let neighbor: Option<(i64, i64)> = if direction == "up" {
        conn.query_row("SELECT id,position FROM task_milestones WHERE task_id=?1 AND position<?2 ORDER BY position DESC LIMIT 1", params![task_id,position], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e| e.to_string())?
    } else {
        conn.query_row("SELECT id,position FROM task_milestones WHERE task_id=?1 AND position>?2 ORDER BY position LIMIT 1", params![task_id,position], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e| e.to_string())?
    };
    if let Some((other_id, other_position)) = neighbor {
        conn.execute(
            "UPDATE task_milestones SET position=?1 WHERE id=?2",
            params![other_position, id],
        )
        .map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE task_milestones SET position=?1 WHERE id=?2",
            params![position, other_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskTemplate {
    pub id: i64,
    pub name: String,
    pub class_id: Option<i64>,
    pub task_type: Option<String>,
    pub default_estimated_minutes: Option<i64>,
    pub default_time_budget_minutes: Option<i64>,
    pub priority: Option<i64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct NewTaskTemplate {
    pub name: String,
    pub class_id: Option<i64>,
    pub task_type: Option<String>,
    pub default_estimated_minutes: Option<i64>,
    pub default_time_budget_minutes: Option<i64>,
    pub priority: Option<i64>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct InstantiateTemplate {
    pub template_id: i64,
    pub class_id: i64,
    pub title: String,
    pub due_at: Option<String>,
    pub scheduled_date: Option<String>,
}

fn template_row(row: &rusqlite::Row) -> rusqlite::Result<TaskTemplate> {
    let tags: String = row.get("tags")?;
    Ok(TaskTemplate {
        id: row.get("id")?,
        name: row.get("name")?,
        class_id: row.get("class_id")?,
        task_type: row.get("task_type")?,
        default_estimated_minutes: row.get("default_estimated_minutes")?,
        default_time_budget_minutes: row.get("default_time_budget_minutes")?,
        priority: row.get("priority")?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
    })
}

#[tauri::command]
pub fn list_task_templates(db: State<Db>) -> Result<Vec<TaskTemplate>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let result = conn
        .prepare("SELECT * FROM task_templates ORDER BY name")
        .map_err(|e| e.to_string())?
        .query_map([], template_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn create_task_template(db: State<Db>, input: NewTaskTemplate) -> Result<TaskTemplate, String> {
    if input.name.trim().is_empty() {
        return Err("Template name cannot be empty.".into());
    }
    if input.default_estimated_minutes.is_some_and(|v| v < 0)
        || input.default_time_budget_minutes.is_some_and(|v| v < 0)
    {
        return Err("Template minutes cannot be negative.".into());
    }
    let tags = serde_json::to_string(&input.tags).map_err(|e| e.to_string())?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("INSERT INTO task_templates(name,class_id,task_type,default_estimated_minutes,default_time_budget_minutes,priority,tags) VALUES(?1,?2,?3,?4,?5,?6,?7)", params![input.name.trim(),input.class_id,input.task_type,input.default_estimated_minutes,input.default_time_budget_minutes,input.priority,tags]).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT * FROM task_templates WHERE id=?1",
        [conn.last_insert_rowid()],
        template_row,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_task_template(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM task_templates WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn instantiate_task_template(
    db: State<Db>,
    input: InstantiateTemplate,
) -> Result<Task, String> {
    if input.title.trim().is_empty() {
        return Err("Task title cannot be empty.".into());
    }
    for value in [&input.due_at, &input.scheduled_date].into_iter().flatten() {
        date(value)?;
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let template: TaskTemplate = conn
        .query_row(
            "SELECT * FROM task_templates WHERE id=?1",
            [input.template_id],
            template_row,
        )
        .map_err(|_| "Template not found")?;
    let class_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM classes WHERE id=?1 AND active=1)",
            [input.class_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !class_exists {
        return Err("Class not found.".into());
    }
    conn.execute("INSERT INTO tasks(class_id,title,due_at,scheduled_date,estimated_minutes,priority,task_type,tags,time_budget_minutes) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![input.class_id,input.title.trim(),input.due_at,input.scheduled_date,template.default_estimated_minutes,template.priority,template.task_type.unwrap_or_else(|| "assignment".into()),serde_json::to_string(&template.tags).map_err(|e| e.to_string())?,template.default_time_budget_minutes]).map_err(|e| e.to_string())?;
    let task_id = conn.last_insert_rowid();
    let due = input
        .due_at
        .as_deref()
        .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok());
    let mut stmt = conn.prepare("SELECT title,offset_days_before_due,position FROM task_template_milestones WHERE template_id=?1 ORDER BY position,id").map_err(|e| e.to_string())?;
    let presets: Vec<(String, Option<i64>, i64)> = stmt
        .query_map([input.template_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    for (title, offset, position) in presets {
        let target = due.and_then(|d| offset.map(|n| (d - Duration::days(n)).to_string()));
        conn.execute(
            "INSERT INTO task_milestones(task_id,title,target_date,position) VALUES(?1,?2,?3,?4)",
            params![task_id, title, target, position],
        )
        .map_err(|e| e.to_string())?;
    }
    conn.query_row(
        &format!("{TASK_SELECT} WHERE id=?1"),
        [task_id],
        row_to_task,
    )
    .map_err(|e| e.to_string())
}
