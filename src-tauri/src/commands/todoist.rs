use crate::db::Db;
use crate::todoist;
use crate::todoist::client::TodoistClient;
use crate::todoist::sync::SyncResult;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct TodoistStatus {
    pub connected: bool,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TodoistProjectMapping {
    pub todoist_id: String,
    pub name: String,
    pub class_id: Option<i64>,
    pub synced_at: Option<String>,
}

#[tauri::command]
pub fn get_todoist_status(db: State<Db>) -> Result<TodoistStatus, String> {
    let connected = todoist::get_token()?.is_some();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let last_synced_at: Option<String> = conn
        .query_row("SELECT MAX(synced_at) FROM todoist_projects", [], |r| {
            r.get(0)
        })
        .unwrap_or(None);
    Ok(TodoistStatus {
        connected,
        last_synced_at,
    })
}

#[tauri::command]
pub fn set_todoist_token(token: String) -> Result<(), String> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return Err("Token cannot be empty".into());
    }
    // Validate before persisting so a typo doesn't get saved as "connected".
    let client = TodoistClient::new(trimmed.to_string());
    client.test_connection().map_err(|e| e.to_string())?;
    todoist::set_token(trimmed)
}

#[tauri::command]
pub fn disconnect_todoist() -> Result<(), String> {
    todoist::clear_token()
}

#[tauri::command]
pub fn list_todoist_project_mappings(db: State<Db>) -> Result<Vec<TodoistProjectMapping>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT todoist_id, name, class_id, synced_at FROM todoist_projects ORDER BY name")
        .map_err(|e| e.to_string())?;
    let result = stmt
        .query_map([], |row| {
            Ok(TodoistProjectMapping {
                todoist_id: row.get(0)?,
                name: row.get(1)?,
                class_id: row.get(2)?,
                synced_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn map_todoist_project(db: State<Db>, todoist_id: String, class_id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE todoist_projects SET class_id = ?1 WHERE todoist_id = ?2",
        rusqlite::params![class_id, todoist_id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE classes SET todoist_project_id = ?1 WHERE id = ?2",
        rusqlite::params![todoist_id, class_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn sync_todoist_now(db: State<Db>) -> Result<SyncResult, String> {
    let token = todoist::get_token()?.ok_or("No Todoist token configured".to_string())?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    todoist::sync::sync_now(&conn, &token).map_err(|e| e.to_string())
}
