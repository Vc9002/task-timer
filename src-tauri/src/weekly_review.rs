use crate::db::Db;
use chrono::{Datelike, Local, Timelike};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const POLL_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WeeklyReviewSettings {
    pub enabled: bool,
    /// 0 = Sunday .. 6 = Saturday, matching chrono's `Weekday::num_days_from_sunday`.
    pub weekday: u32,
    pub hour: u32,
}

impl Default for WeeklyReviewSettings {
    fn default() -> Self {
        WeeklyReviewSettings {
            enabled: false,
            weekday: 0,
            hour: 18,
        }
    }
}

fn settings(conn: &rusqlite::Connection) -> WeeklyReviewSettings {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key='weekly_review_v01'",
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
    settings: &WeeklyReviewSettings,
) -> Result<(), String> {
    if settings.weekday > 6 {
        return Err("Choose a valid day of the week.".into());
    }
    if settings.hour > 23 {
        return Err("Choose a valid hour (0-23).".into());
    }
    let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES('weekly_review_v01',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_weekly_review_settings(db: tauri::State<Db>) -> Result<WeeklyReviewSettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(settings(&conn))
}

#[tauri::command]
pub fn save_weekly_review_settings(
    db: tauri::State<Db>,
    settings: WeeklyReviewSettings,
) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    save_settings(&conn, &settings)
}

fn already_notified_this_week(conn: &rusqlite::Connection, iso_week_key: &str) -> bool {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key='weekly_review_last_notified'",
        [],
        |r| r.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .as_deref()
        == Some(iso_week_key)
}

fn mark_notified(conn: &rusqlite::Connection, iso_week_key: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES('weekly_review_last_notified',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [iso_week_key],
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
    let now = Local::now();
    if now.weekday().num_days_from_sunday() != prefs.weekday || now.hour() != prefs.hour {
        return Ok(());
    }
    let iso = now.iso_week();
    let week_key = format!("{}-W{:02}", iso.year(), iso.week());
    if already_notified_this_week(&conn, &week_key) {
        return Ok(());
    }
    mark_notified(&conn, &week_key)?;
    drop(conn);
    if app
        .notification()
        .request_permission()
        .map(|state| state == tauri::plugin::PermissionState::Granted)
        .unwrap_or(false)
    {
        let _ = app
            .notification()
            .builder()
            .title("TaskTimer")
            .body("Your weekly review is ready. Check Analytics for what changed this week.")
            .show();
    }
    Ok(())
}

pub fn setup(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        if let Err(error) = check(&app) {
            eprintln!("Weekly review check: {error}");
        }
        std::thread::sleep(POLL_INTERVAL);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_disabled_sunday_evening() {
        let conn = crate::db::test_connection();
        let s = settings(&conn);
        assert!(!s.enabled);
        assert_eq!(s.weekday, 0);
        assert_eq!(s.hour, 18);
    }

    #[test]
    fn saved_settings_round_trip() {
        let conn = crate::db::test_connection();
        save_settings(
            &conn,
            &WeeklyReviewSettings {
                enabled: true,
                weekday: 5,
                hour: 9,
            },
        )
        .unwrap();
        let s = settings(&conn);
        assert!(s.enabled);
        assert_eq!(s.weekday, 5);
        assert_eq!(s.hour, 9);
    }

    #[test]
    fn rejects_invalid_weekday_or_hour() {
        let conn = crate::db::test_connection();
        assert!(save_settings(
            &conn,
            &WeeklyReviewSettings {
                enabled: true,
                weekday: 7,
                hour: 9
            }
        )
        .is_err());
        assert!(save_settings(
            &conn,
            &WeeklyReviewSettings {
                enabled: true,
                weekday: 0,
                hour: 24
            }
        )
        .is_err());
    }

    #[test]
    fn notifies_once_per_iso_week() {
        let conn = crate::db::test_connection();
        assert!(!already_notified_this_week(&conn, "2026-W38"));
        mark_notified(&conn, "2026-W38").unwrap();
        assert!(already_notified_this_week(&conn, "2026-W38"));
        assert!(!already_notified_this_week(&conn, "2026-W39"));
    }
}
