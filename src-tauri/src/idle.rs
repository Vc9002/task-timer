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
#[cfg(any(target_os = "macos", test))]
fn parse_idle_seconds(ioreg_output: &str) -> Option<f64> {
    let line = ioreg_output.lines().find(|l| l.contains("HIDIdleTime"))?;
    let ns: u64 = line.split('=').nth(1)?.trim().parse().ok()?;
    Some(ns as f64 / 1_000_000_000.0)
}

#[cfg(target_os = "macos")]
fn system_idle_seconds() -> Option<f64> {
    let output = std::process::Command::new("ioreg")
        .args(["-c", "IOHIDSystem", "-d", "4"])
        .output()
        .ok()?;
    parse_idle_seconds(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "windows")]
fn system_idle_seconds() -> Option<f64> {
    use windows_sys::Win32::System::SystemInformation::GetTickCount64;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

    let mut last_input = LASTINPUTINFO {
        cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // GetLastInputInfo returns a 32-bit tick count. Wrapping subtraction keeps
    // elapsed time valid after the Windows tick counter rolls over.
    if unsafe { GetLastInputInfo(&mut last_input) } == 0 {
        return None;
    }
    let now = unsafe { GetTickCount64() } as u32;
    Some(windows_idle_seconds(now, last_input.dwTime))
}

#[cfg(any(target_os = "windows", test))]
fn windows_idle_seconds(now: u32, last_input: u32) -> f64 {
    now.wrapping_sub(last_input) as f64 / 1000.0
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn system_idle_seconds() -> Option<f64> {
    None
}

/// How long the user has been away, in seconds, given the OS idle time and the
/// wall-clock gap since the previous poll.
///
/// A gap far longer than the poll interval means the machine was asleep. The
/// OS idle counter is useless then, because the keypress that woke the machine
/// resets it, so the whole gap counts as time away.
fn away_seconds(idle_seconds: Option<f64>, since_last_poll: Duration) -> f64 {
    let asleep = if since_last_poll > POLL_INTERVAL * 3 {
        since_last_poll.as_secs_f64()
    } else {
        0.0
    };
    idle_seconds.unwrap_or(0.0).max(asleep)
}

fn check(app: &AppHandle, since_last_poll: Duration) -> Result<(), String> {
    let db = app.state::<Db>();
    let (threshold, session_id) = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        let threshold = threshold_minutes(&conn);
        if threshold == 0 {
            return Ok(());
        }
        let Some(active) = timer::core_get_active_session(&conn).map_err(|e| format!("{e:?}"))?
        else {
            return Ok(());
        };
        if active.is_paused {
            return Ok(());
        }
        (threshold, active.session.id)
    };
    let away = away_seconds(system_idle_seconds(), since_last_poll);
    if away < (threshold * 60) as f64 {
        return Ok(());
    }
    // Pause as of when the user left, not as of now, so the time away is not
    // counted as work. The update is a no-op if the session changed meanwhile.
    let paused = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        timer::core_pause_timer_backdated(&conn, session_id, away as i64)
            .map_err(|e| e.to_string())?
    };
    if paused {
        crate::tray::changed(app);
    }
    Ok(())
}

pub fn setup(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        // Wall-clock time, not Instant: Instant may not advance while the machine sleeps.
        let mut last_poll = std::time::SystemTime::now();
        loop {
            std::thread::sleep(POLL_INTERVAL);
            let now = std::time::SystemTime::now();
            let since_last_poll = now.duration_since(last_poll).unwrap_or_default();
            last_poll = now;
            if let Err(error) = check(&app, since_last_poll) {
                eprintln!("Idle check: {error}");
            }
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

    #[test]
    fn windows_tick_rollover_preserves_idle_duration() {
        assert_eq!(windows_idle_seconds(1000, u32::MAX - 999), 2.0);
    }

    #[test]
    fn away_time_is_the_idle_time_during_normal_polling() {
        assert_eq!(away_seconds(Some(400.0), POLL_INTERVAL), 400.0);
        assert_eq!(away_seconds(None, POLL_INTERVAL), 0.0);
    }

    #[test]
    fn a_long_gap_between_polls_counts_as_sleep_even_with_fresh_input() {
        // Woke from a two-hour sleep by pressing a key, so the OS reports 1s idle.
        let gap = Duration::from_secs(7200);
        assert_eq!(away_seconds(Some(1.0), gap), 7200.0);
        assert_eq!(away_seconds(None, gap), 7200.0);
        // A slightly late poll is not sleep.
        assert_eq!(
            away_seconds(Some(1.0), POLL_INTERVAL + Duration::from_secs(5)),
            1.0
        );
    }
}
