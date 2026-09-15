use crate::{db::Db, timer};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub enabled: bool,
    pub overrun_percent: u32,
}
#[derive(Default)]
pub struct Wake {
    generation: Mutex<u64>,
    changed: Condvar,
}
pub struct Reminders(pub Arc<Wake>);
pub fn wake(app: &AppHandle) {
    if let Some(state) = app.try_state::<Reminders>() {
        if let Ok(mut generation) = state.0.generation.lock() {
            *generation += 1;
            state.0.changed.notify_one();
        }
    }
}
fn preferences(conn: &rusqlite::Connection) -> Result<Preferences, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='overrun_v02'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| "Couldn't load notification settings")?;
    value
        .map(|s| serde_json::from_str(&s).map_err(|_| "Couldn't read notification settings".into()))
        .unwrap_or(Ok(Preferences {
            enabled: false,
            overrun_percent: 25,
        }))
}
#[tauri::command]
pub fn notification_settings(db: tauri::State<Db>) -> Result<Preferences, String> {
    let conn =
        db.0.lock()
            .map_err(|_| "Couldn't load notification settings")?;
    preferences(&conn)
}
#[tauri::command]
pub async fn save_notification_settings(
    app: AppHandle,
    settings: Preferences,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if ![0,25,50].contains(&settings.overrun_percent) { return Err("Choose a supported estimate threshold.".into()); }
        if settings.enabled && app.notification().request_permission().map_err(|_| "Couldn't request notification permission")? != tauri::plugin::PermissionState::Granted { return Err("Notifications are blocked. Enable them in system settings.".into()); }
        {
            let db = app.state::<Db>(); let conn = db.0.lock().map_err(|_| "Couldn't save notification settings")?;
            conn.execute("INSERT INTO app_settings(key,value) VALUES('overrun_v02',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&settings).map_err(|_| "Couldn't save notification settings")?]).map_err(|_| "Couldn't save notification settings")?;
        }
        wake(&app); Ok(())
    }).await.map_err(|_| "Couldn't save notification settings")?
}
#[tauri::command]
pub fn test_notification(app: AppHandle) -> Result<(), String> {
    let enabled = {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| "Couldn't load settings")?;
        preferences(&conn)?.enabled
    };
    if !enabled {
        return Err("Enable notifications first.".into());
    }
    app.notification()
        .builder()
        .title("TaskTimer")
        .body("Notifications are enabled. Your timer and tasks stay local.")
        .show()
        .map_err(|_| {
            "Couldn't deliver the notification. Check system notification settings.".into()
        })
}

fn remaining(estimate: i64, percent: u32, direct_finished: i64, elapsed: i64) -> i64 {
    estimate
        .saturating_mul(60)
        .saturating_mul(100 + i64::from(percent))
        / 100
        - direct_finished.saturating_add(elapsed)
}

// Returns the next wait duration. None means no repeating work until a state change.
fn check(app: &AppHandle) -> Result<Option<Duration>, String> {
    let notification = {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| "Reminder database unavailable")?;
        let settings = preferences(&conn)?;
        if !settings.enabled {
            return Ok(None);
        }
        let Some(active) =
            timer::core_get_active_session(&conn).map_err(|_| "Couldn't read timer")?
        else {
            return Ok(None);
        };
        if active.is_paused {
            return Ok(None);
        }
        let (estimate,finished): (Option<i64>,i64)=conn.query_row("SELECT estimated_minutes,(SELECT COALESCE(SUM(final_duration_seconds),0) FROM time_sessions WHERE task_id=t.id AND end_ts IS NOT NULL) FROM tasks t WHERE t.id=?1",[active.session.task_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_| "Couldn't read estimate")?;
        let Some(estimate) = estimate.filter(|n| *n > 0) else {
            return Ok(None);
        };
        let key = format!(
            "overrun_sent_{}_{}",
            active.session.id, settings.overrun_percent
        );
        let sent: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key=?1)",
                [&key],
                |r| r.get(0),
            )
            .map_err(|_| "Couldn't read reminder state")?;
        if sent {
            return Ok(None);
        }
        let seconds = remaining(
            estimate,
            settings.overrun_percent,
            finished,
            active.elapsed_seconds,
        );
        if seconds > 0 {
            return Ok(Some(Duration::from_secs(seconds as u64)));
        }
        // Claim before delivery: at most one attempt even across restart or OS failure.
        conn.execute(
            "INSERT INTO app_settings(key,value) VALUES(?1,'attempted')",
            [key],
        )
        .map_err(|_| "Couldn't save reminder state")?;
        format!(
            "{} has passed its {}m estimate. Tracked directly: {}m.",
            active.task_title,
            estimate,
            (finished + active.elapsed_seconds) / 60
        )
    };
    app.notification()
        .builder()
        .title("TaskTimer")
        .body(notification)
        .show()
        .map_err(|_| "Overrun notification delivery failed")?;
    Ok(None)
}
pub fn setup(app: &AppHandle) {
    let state = Arc::new(Wake::default());
    app.manage(Reminders(state.clone()));
    let app = app.clone();
    std::thread::spawn(move || loop {
        let observed = match state.generation.lock() {
            Ok(g) => *g,
            Err(_) => return,
        };
        let delay = match check(&app) {
            Ok(delay) => delay,
            Err(error) => {
                eprintln!("{error}");
                None
            }
        };
        let mut generation = match state.generation.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if *generation != observed {
            continue;
        }
        if let Some(delay) = delay {
            if state
                .changed
                .wait_timeout_while(generation, delay, |g| *g == observed)
                .is_err()
            {
                return;
            }
        } else {
            while *generation == observed {
                generation = match state.changed.wait(generation) {
                    Ok(g) => g,
                    Err(_) => return,
                };
            }
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overrun_uses_direct_work_and_threshold() {
        assert_eq!(remaining(40, 25, 600, 1200), 1200);
        assert_eq!(remaining(40, 0, 600, 1800), 0);
        assert!(remaining(40, 50, 0, 4000) < 0);
    }
    #[test]
    fn notifications_default_off() {
        assert!(!preferences(&crate::db::test_connection()).unwrap().enabled);
    }
}
