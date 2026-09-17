use crate::db::Db;
use crate::todoist;
use crate::todoist::client::TodoistClient;
use crate::todoist::sync::SyncResult;
use serde::Serialize;
use tauri::{Manager, State};

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

#[derive(Debug, Serialize)]
pub struct TodoistOutboxEntry {
    pub id: i64,
    pub external_id: String,
    pub status: String,
    pub attempts: i64,
    pub last_error: Option<String>,
    pub updated_at: String,
}

#[tauri::command]
pub fn get_todoist_outbox_status(db: State<Db>) -> Result<Vec<TodoistOutboxEntry>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let result = conn.prepare("SELECT id,external_id,status,attempts,last_error,updated_at FROM integration_outbox ORDER BY id DESC LIMIT 25")
        .map_err(|e| e.to_string())?
        .query_map([], |row| Ok(TodoistOutboxEntry { id: row.get(0)?, external_id: row.get(1)?, status: row.get(2)?, attempts: row.get(3)?, last_error: row.get(4)?, updated_at: row.get(5)? }))
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>().map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn get_todoist_status(db: State<Db>) -> Result<TodoistStatus, String> {
    let connected = todoist::get_token()?.is_some();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let last_synced_at: Option<String> = conn
        .query_row(
            "SELECT value FROM sync_metadata WHERE key='todoist_last_sync'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(None);
    Ok(TodoistStatus {
        connected,
        last_synced_at,
    })
}

#[tauri::command]
pub async fn set_todoist_token(app: tauri::AppHandle, token: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _gate = todoist::sync::gate()?;
        let trimmed = token.trim();
        if trimmed.is_empty() {
            return Err("Token cannot be empty".into());
        }
        TodoistClient::new(trimmed.to_string())
            .map_err(|e| e.to_string())?
            .test_connection()
            .map_err(|e| e.to_string())?;
        // Reset cursor before changing account; a failed keychain write merely causes a full sync.
        let db = app.state::<Db>();
        let conn =
            db.0.lock()
                .map_err(|_| "Couldn't access local Todoist settings")?;
        conn.execute("DELETE FROM sync_metadata WHERE key='todoist_cursor'", [])
            .map_err(|_| "Couldn't reset Todoist sync")?;
        todoist::set_token(trimmed)
    })
    .await
    .map_err(|_| "Couldn't connect Todoist. Try again.".to_string())?
}

#[tauri::command]
pub async fn disconnect_todoist() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let _gate = todoist::sync::gate()?;
        todoist::clear_token()
    })
    .await
    .map_err(|_| "Couldn't disconnect Todoist. Try again.".to_string())?
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
pub fn map_todoist_project(
    db: State<Db>,
    todoist_id: String,
    class_id: Option<i64>,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|_| "Couldn't access local tasks")?;
    todoist::sync::map_project(&conn, &todoist_id, class_id)
        .map_err(|_| "Couldn't save the project mapping. Try again.".into())
}

#[tauri::command]
pub async fn sync_todoist_now(app: tauri::AppHandle) -> Result<SyncResult, String> {
    tauri::async_runtime::spawn_blocking(move || todoist::sync::sync_now(&app.state::<Db>()))
        .await
        .map_err(|_| {
            "Todoist sync couldn't finish. Your local tasks and timer still work.".to_string()
        })?
}

#[tauri::command]
pub async fn complete_todoist_task(app: tauri::AppHandle, task_id: i64) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = app.state::<Db>();
        {
            let conn = db.0.lock().map_err(|_| "Couldn't access local tasks")?;
            crate::todoist::sync::queue_completion(&conn, task_id)?;
        }
        // Local completion is durable even while Todoist is offline. The queued
        // command is retried by the next explicit or startup sync.
        let _ = crate::todoist::sync::flush_outbox(&db);
        Ok::<(), String>(())
    })
    .await
    .map_err(|_| "Couldn't complete the Todoist task.".to_string())?
}
