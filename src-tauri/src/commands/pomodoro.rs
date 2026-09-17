use crate::db::Db;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_notification::NotificationExt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PomodoroSettings {
    pub work_minutes: i64,
    pub break_minutes: i64,
}

impl Default for PomodoroSettings {
    fn default() -> Self {
        PomodoroSettings {
            work_minutes: 25,
            break_minutes: 5,
        }
    }
}

fn pomodoro_settings(conn: &Connection) -> rusqlite::Result<PomodoroSettings> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='pomodoro_v01'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    Ok(value
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

fn save_settings(conn: &Connection, settings: &PomodoroSettings) -> Result<(), String> {
    if settings.work_minutes < 1 || settings.work_minutes > 180 {
        return Err("Work length must be between 1 and 180 minutes.".into());
    }
    if settings.break_minutes < 1 || settings.break_minutes > 60 {
        return Err("Break length must be between 1 and 60 minutes.".into());
    }
    let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES('pomodoro_v01',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_pomodoro_settings(db: State<Db>) -> Result<PomodoroSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    pomodoro_settings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_pomodoro_settings(db: State<Db>, settings: PomodoroSettings) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    save_settings(&conn, &settings)
}

/// Fires a local notification for a Pomodoro phase change. Silently succeeds
/// if the general notification permission isn't granted, since the Pomodoro
/// clock keeps running visually either way.
#[tauri::command]
pub fn notify_pomodoro_phase(app: AppHandle, phase: String) -> Result<(), String> {
    let (title, body) = match phase.as_str() {
        "work_done" => ("Pomodoro: work session done", "Time for a break."),
        "break_done" => ("Pomodoro: break's over", "Back to work."),
        _ => return Err("Unknown Pomodoro phase.".into()),
    };
    if app
        .notification()
        .request_permission()
        .map(|state| state == tauri::plugin::PermissionState::Granted)
        .unwrap_or(false)
    {
        let _ = app.notification().builder().title(title).body(body).show();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        crate::db::test_connection()
    }

    #[test]
    fn defaults_are_25_and_5_when_unset() {
        let conn = db();
        let settings = pomodoro_settings(&conn).unwrap();
        assert_eq!(settings.work_minutes, 25);
        assert_eq!(settings.break_minutes, 5);
    }

    #[test]
    fn saved_settings_round_trip() {
        let conn = db();
        save_settings(
            &conn,
            &PomodoroSettings {
                work_minutes: 50,
                break_minutes: 10,
            },
        )
        .unwrap();
        let settings = pomodoro_settings(&conn).unwrap();
        assert_eq!(settings.work_minutes, 50);
        assert_eq!(settings.break_minutes, 10);
    }

    #[test]
    fn rejects_out_of_range_values() {
        let conn = db();
        assert!(save_settings(
            &conn,
            &PomodoroSettings {
                work_minutes: 0,
                break_minutes: 5
            }
        )
        .is_err());
        assert!(save_settings(
            &conn,
            &PomodoroSettings {
                work_minutes: 25,
                break_minutes: 0
            }
        )
        .is_err());
    }
}
