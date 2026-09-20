use crate::db::Db;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

const POLL_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// Keep the most recent N auto-backups so the folder doesn't grow forever.
const KEEP_COUNT: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CloudBackupSettings {
    pub enabled: bool,
    /// Absolute path to a folder (e.g. inside iCloud Drive) to write timestamped
    /// backup snapshots into. None until the user picks one.
    pub folder: Option<String>,
}

impl Default for CloudBackupSettings {
    fn default() -> Self {
        CloudBackupSettings {
            enabled: false,
            folder: default_icloud_folder(),
        }
    }
}

/// Best-effort default: macOS's iCloud Drive app-storage container, if present.
/// Purely a convenience suggestion — the user can always pick a different folder.
fn default_icloud_folder() -> Option<String> {
    let home = dirs_home()?;
    let icloud = home.join("Library/Mobile Documents/com~apple~CloudDocs/TaskTimer Backups");
    Some(icloud.to_string_lossy().into_owned())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn settings(conn: &rusqlite::Connection) -> CloudBackupSettings {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key='cloud_backup_v01'",
        [],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

fn save_settings(
    conn: &rusqlite::Connection,
    settings: &CloudBackupSettings,
) -> Result<(), String> {
    if settings.enabled && settings.folder.as_deref().is_none_or(str::is_empty) {
        return Err("Choose a backup folder first.".into());
    }
    let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES('cloud_backup_v01',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_cloud_backup_settings(db: tauri::State<Db>) -> Result<CloudBackupSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(settings(&conn))
}

#[tauri::command]
pub fn save_cloud_backup_settings(
    db: tauri::State<Db>,
    settings: CloudBackupSettings,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    save_settings(&conn, &settings)
}

#[tauri::command]
pub async fn pick_cloud_backup_folder(app: AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(app
            .dialog()
            .file()
            .blocking_pick_folder()
            .and_then(|f| f.into_path().ok())
            .map(|p| p.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|_| "Couldn't open folder picker".to_string())?
}

#[tauri::command]
pub fn run_cloud_backup_now(app: AppHandle) -> Result<String, String> {
    let db = app.state::<Db>();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let prefs = settings(&conn);
    let folder = prefs.folder.ok_or("Choose a backup folder first.")?;
    write_backup(&conn, &folder)
}

fn write_backup(conn: &rusqlite::Connection, folder: &str) -> Result<String, String> {
    let dir = PathBuf::from(folder);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create backup folder: {e}"))?;
    let bytes = crate::export::backup(conn)?;
    let filename = format!(
        "task-timer-{}.json",
        chrono::Local::now().format("%Y-%m-%d_%H%M%S")
    );
    let path = dir.join(&filename);
    std::fs::write(&path, bytes).map_err(|e| format!("Couldn't write backup file: {e}"))?;
    prune_old_backups(&dir);
    Ok(path.to_string_lossy().into_owned())
}

fn prune_old_backups(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("task-timer-"))
        .collect();
    files.sort_by_key(|e| e.file_name());
    if files.len() <= KEEP_COUNT {
        return;
    }
    for entry in &files[..files.len() - KEEP_COUNT] {
        let _ = std::fs::remove_file(entry.path());
    }
}

fn already_ran_today(conn: &rusqlite::Connection, key: &str) -> bool {
    conn.query_row("SELECT value FROM app_settings WHERE key=?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .ok()
    .flatten()
    .is_some()
}

fn mark_ran(conn: &rusqlite::Connection, key: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,'1')
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [key],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn check(app: &AppHandle) -> Result<(), String> {
    let db = app.state::<Db>();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let prefs = settings(&conn);
    if !prefs.enabled {
        return Ok(());
    }
    let Some(folder) = prefs.folder.clone() else {
        return Ok(());
    };
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let key = format!("cloud_backup_last_run_{today}");
    if already_ran_today(&conn, &key) {
        return Ok(());
    }
    write_backup(&conn, &folder)?;
    mark_ran(&conn, &key)?;
    Ok(())
}

pub fn setup(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        if let Err(error) = check(&app) {
            eprintln!("Cloud backup check: {error}");
        }
        std::thread::sleep(POLL_INTERVAL);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_disabled_with_no_chosen_folder_override() {
        let conn = crate::db::test_connection();
        let s = settings(&conn);
        assert!(!s.enabled);
    }

    #[test]
    fn rejects_enabling_without_a_folder() {
        let conn = crate::db::test_connection();
        assert!(save_settings(
            &conn,
            &CloudBackupSettings {
                enabled: true,
                folder: None
            }
        )
        .is_err());
    }

    #[test]
    fn saved_settings_round_trip() {
        let conn = crate::db::test_connection();
        save_settings(
            &conn,
            &CloudBackupSettings {
                enabled: true,
                folder: Some("/tmp/backups".into()),
            },
        )
        .unwrap();
        let s = settings(&conn);
        assert!(s.enabled);
        assert_eq!(s.folder.as_deref(), Some("/tmp/backups"));
    }

    #[test]
    fn writes_and_prunes_backups() {
        let conn = crate::db::test_connection();
        let dir = std::env::temp_dir().join(format!("tt-cloud-backup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for _ in 0..3 {
            write_backup(&conn, dir.to_str().unwrap()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(1100));
        }
        let count = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_once_per_day() {
        let conn = crate::db::test_connection();
        let key = "cloud_backup_last_run_2026-09-20";
        assert!(!already_ran_today(&conn, key));
        mark_ran(&conn, key).unwrap();
        assert!(already_ran_today(&conn, key));
    }
}
