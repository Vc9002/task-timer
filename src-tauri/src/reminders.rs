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
        if crate::focus_mode::is_active(app) {
            // Defer rather than consume the notification: it should still
            // fire once Focus Mode is turned off.
            return Ok(Some(Duration::from_secs(5 * 60)));
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
        let sent: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM session_notifications WHERE session_id=?1 AND notification_type='overrun' AND threshold=?2 AND status='sent')",
                rusqlite::params![active.session.id,settings.overrun_percent],
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
        conn.execute(
            "INSERT INTO session_notifications(session_id,notification_type,threshold,status,attempts,attempted_at) VALUES(?1,'overrun',?2,'attempted',1,datetime('now')) ON CONFLICT(session_id,notification_type,threshold) DO UPDATE SET status='attempted',attempts=attempts+1,attempted_at=datetime('now')",
            rusqlite::params![active.session.id,settings.overrun_percent],
        )
        .map_err(|_| "Couldn't save reminder state")?;
        (
            active.session.id,
            settings.overrun_percent,
            format!(
                "{} has passed its {}m estimate. Tracked directly: {}m.",
                active.task_title,
                estimate,
                (finished + active.elapsed_seconds) / 60
            ),
        )
    };
    let delivered = app
        .notification()
        .builder()
        .title("TaskTimer")
        .body(notification.2)
        .show()
        .is_ok();
    {
        let db = app.state::<Db>();
        let conn =
            db.0.lock()
                .map_err(|_| "Couldn't record notification result")?;
        record_result(&conn, notification.0, notification.1, delivered)
            .map_err(|_| "Couldn't record notification result")?;
    }
    // Failures retry on the next state/focus change or startup, never a polling loop.
    if !delivered {
        return Err("Overrun notification delivery failed; will retry on next state change".into());
    }
    Ok(None)
}

fn record_result(
    conn: &rusqlite::Connection,
    session_id: i64,
    threshold: u32,
    sent: bool,
) -> rusqlite::Result<()> {
    conn.execute("UPDATE session_notifications SET status=?3,sent_at=CASE WHEN ?3='sent' THEN datetime('now') ELSE NULL END WHERE session_id=?1 AND notification_type='overrun' AND threshold=?2",rusqlite::params![session_id,threshold,if sent { "sent" } else { "failed" }])?;
    Ok(())
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
    fn delivery_failure_can_retry_and_cancel_cleans_only_notification_state() {
        let conn = crate::db::test_connection();
        conn.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall'); INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Read'); INSERT INTO time_sessions(id,task_id,start_ts) VALUES(1,1,'2026-09-15'); INSERT INTO session_notifications(session_id,notification_type,threshold,status,attempts) VALUES(1,'overrun',25,'attempted',1);").unwrap();
        record_result(&conn, 1, 25, false).unwrap();
        assert_eq!(
            conn.query_row("SELECT status FROM session_notifications", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "failed"
        );
        record_result(&conn, 1, 25, true).unwrap();
        assert!(conn
            .query_row(
                "SELECT sent_at IS NOT NULL FROM session_notifications",
                [],
                |r| r.get::<_, bool>(0)
            )
            .unwrap());
        conn.execute("DELETE FROM time_sessions WHERE id=1", [])
            .unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM session_notifications", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
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
