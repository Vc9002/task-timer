use crate::db::Db;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct Class {
    pub id: i64,
    pub course_code: String,
    pub name: Option<String>,
    pub semester: String,
    pub color: Option<String>,
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewClass {
    pub course_code: String,
    pub name: Option<String>,
    pub semester: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClass {
    pub id: i64,
    pub course_code: String,
    pub name: Option<String>,
    pub semester: String,
    pub color: Option<String>,
    pub active: bool,
}

fn row_to_class(row: &rusqlite::Row) -> rusqlite::Result<Class> {
    Ok(Class {
        id: row.get("id")?,
        course_code: row.get("course_code")?,
        name: row.get("name")?,
        semester: row.get("semester")?,
        color: row.get("color")?,
        active: row.get::<_, i64>("active")? != 0,
    })
}

#[tauri::command]
pub fn list_classes(db: State<Db>, include_inactive: bool) -> Result<Vec<Class>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let sql = if include_inactive {
        "SELECT * FROM classes ORDER BY course_code"
    } else {
        "SELECT * FROM classes WHERE active = 1 ORDER BY course_code"
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let classes = stmt
        .query_map([], row_to_class)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(classes)
}

#[tauri::command]
pub fn create_class(db: State<Db>, input: NewClass) -> Result<Class, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO classes (course_code, name, semester, color) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![input.course_code, input.name, input.semester, input.color],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row("SELECT * FROM classes WHERE id = ?1", [id], row_to_class)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_class(db: State<Db>, input: UpdateClass) -> Result<Class, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE classes SET course_code = ?1, name = ?2, semester = ?3, color = ?4,
         active = ?5, updated_at = datetime('now') WHERE id = ?6",
        rusqlite::params![
            input.course_code,
            input.name,
            input.semester,
            input.color,
            input.active as i64,
            input.id
        ],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT * FROM classes WHERE id = ?1",
        [input.id],
        row_to_class,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn archive_class(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE classes SET active = 0, updated_at = datetime('now') WHERE id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
