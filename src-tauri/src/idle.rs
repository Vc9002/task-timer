use crate::{db::Db, timer};
use rusqlite::OptionalExtension;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const DEFAULT_THRESHOLD_MINUTES: i64 = 5;

fn threshold_minutes(conn: &rusqlite::Connection) -> i64 {
    conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key='idle_threshold_minutes'",
        [],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
    .unwrap_or(DEFAULT_THRESHOLD_MINUTES)
}

#[tauri::command]
pub fn get_idle_threshold(db: tauri::State<Db>) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    Ok(threshold_minutes(&conn))
}

#[tauri::command]
pub fn save_idle_threshold(db: tauri::State<Db>, minutes: i64) -> Result<(), String> {
    if !(0..=120).contains(&minutes) {
        return Err("Idle threshold must be between 0 (off) and 120 minutes.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES('idle_threshold_minutes',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [minutes.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Parses the `HIDIdleTime` line from `ioreg -c IOHIDSystem -d 4` output,
/// which reports nanoseconds since the last keyboard/mouse event.
fn parse_idle_seconds(ioreg_output: &str) -> Option<f64> {
    let line = ioreg_output.lines().find(|l| l.contains("HIDIdleTime"))?;
    let ns: u64 = line.split('=').nth(1)?.trim().parse().ok()?;
    Some(ns as f64 / 1_000_000_000.0)
}

fn system_idle_seconds() -> Option<f64> {
    let output = std::process::Command::new("ioreg")
        .args(["-c", "IOHIDSystem", "-d", "4"])
        .output()
        .ok()?;
    parse_idle_seconds(&String::from_utf8_lossy(&output.stdout))
}

fn check(app: &AppHandle) -> Result<(), String> {
    let db = app.state::<Db>();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let threshold = threshold_minutes(&conn);
    if threshold == 0 {
        return Ok(());
    }
    let Some(active) = timer::core_get_active_session(&conn).map_err(|e| format!("{e:?}"))? else {
        return Ok(());
    };
    if active.is_paused {
        return Ok(());
    }
    drop(conn);
    let Some(idle_seconds) = system_idle_seconds() else {
        return Ok(());
    };
    if idle_seconds < (threshold * 60) as f64 {
        return Ok(());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    timer::core_pause_timer(&conn).map_err(|e| format!("{e:?}"))?;
    drop(conn);
    crate::tray::changed(app);
    Ok(())
}

pub fn setup(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(POLL_INTERVAL);
        if let Err(error) = check(&app) {
            eprintln!("Idle check: {error}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nanoseconds_from_ioreg_line() {
        let sample = "    | |     \"HIDIdleTime\" = 28324375\n";
        assert_eq!(
            parse_idle_seconds(sample),
            Some(28324375.0 / 1_000_000_000.0)
        );
    }

    #[test]
    fn returns_none_when_field_missing() {
        assert_eq!(parse_idle_seconds("no relevant fields here"), None);
    }

    #[test]
    fn threshold_defaults_to_five_minutes() {
        let conn = crate::db::test_connection();
        assert_eq!(threshold_minutes(&conn), 5);
    }
}
