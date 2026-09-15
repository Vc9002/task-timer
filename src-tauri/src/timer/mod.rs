use crate::db::Db;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize, PartialEq)]
pub struct Session {
    pub id: i64,
    pub task_id: i64,
    pub start_ts: String,
    pub end_ts: Option<String>,
    pub accumulated_pause_seconds: i64,
    pub pause_started_ts: Option<String>,
    pub final_duration_seconds: Option<i64>,
    pub edited: bool,
}

#[derive(Debug, Serialize)]
pub struct ActiveSessionInfo {
    pub session: Session,
    pub task_title: String,
    pub class_course_code: String,
    pub is_paused: bool,
    pub elapsed_seconds: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(tag = "kind")]
pub enum TimerError {
    /// Another task is already being tracked; the frontend renders the
    /// "You're currently tracking X" dialog from this, not a generic error.
    ActiveSessionConflict {
        task_id: i64,
        task_title: String,
        class_course_code: String,
    },
    NotFound,
    Other {
        message: String,
    },
}

impl From<rusqlite::Error> for TimerError {
    fn from(e: rusqlite::Error) -> Self {
        TimerError::Other {
            message: e.to_string(),
        }
    }
}

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get("id")?,
        task_id: row.get("task_id")?,
        start_ts: row.get("start_ts")?,
        end_ts: row.get("end_ts")?,
        accumulated_pause_seconds: row.get("accumulated_pause_seconds")?,
        pause_started_ts: row.get("pause_started_ts")?,
        final_duration_seconds: row.get("final_duration_seconds")?,
        edited: row.get::<_, i64>("edited")? != 0,
    })
}

fn find_active_session(conn: &Connection) -> rusqlite::Result<Option<Session>> {
    conn.query_row(
        "SELECT * FROM time_sessions WHERE end_ts IS NULL LIMIT 1",
        [],
        row_to_session,
    )
    .optional()
}

fn task_and_class_label(conn: &Connection, task_id: i64) -> rusqlite::Result<(String, String)> {
    conn.query_row(
        "SELECT t.title, c.course_code FROM tasks t JOIN classes c ON c.id = t.class_id
         WHERE t.id = ?1",
        [task_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
}

/// now - start - accumulated_pause - (elapsed of the current pause, if paused).
/// Always recomputed from stored timestamps — never a running in-memory counter,
/// so app restarts, sleep, or a stalled UI tick can never desync the value.
fn compute_elapsed_seconds(conn: &Connection, session: &Session) -> rusqlite::Result<i64> {
    let now_minus_start: i64 = conn.query_row(
        "SELECT MAX(0, unixepoch('now') - unixepoch(?1))",
        [&session.start_ts],
        |r| r.get(0),
    )?;
    let mut elapsed = now_minus_start - session.accumulated_pause_seconds;
    if let Some(pause_started) = &session.pause_started_ts {
        let ongoing_pause: i64 = conn.query_row(
            "SELECT MAX(0, unixepoch('now') - unixepoch(?1))",
            [pause_started],
            |r| r.get(0),
        )?;
        elapsed -= ongoing_pause;
    }
    Ok(elapsed.max(0))
}

fn build_active_info(conn: &Connection, session: Session) -> rusqlite::Result<ActiveSessionInfo> {
    let (task_title, class_course_code) = task_and_class_label(conn, session.task_id)?;
    let elapsed_seconds = compute_elapsed_seconds(conn, &session)?;
    let is_paused = session.pause_started_ts.is_some();
    Ok(ActiveSessionInfo {
        session,
        task_title,
        class_course_code,
        is_paused,
        elapsed_seconds,
    })
}

// ---------------------------------------------------------------------------
// Core logic — plain functions over &Connection, independent of Tauri State,
// so they can be exercised directly by tests without a running app.
// ---------------------------------------------------------------------------

pub fn core_get_active_session(conn: &Connection) -> Result<Option<ActiveSessionInfo>, TimerError> {
    match find_active_session(conn)? {
        Some(session) => Ok(Some(build_active_info(conn, session)?)),
        None => Ok(None),
    }
}

pub fn core_start_timer(conn: &Connection, task_id: i64) -> Result<ActiveSessionInfo, TimerError> {
    if let Some(existing) = find_active_session(conn)? {
        let (task_title, class_course_code) = task_and_class_label(conn, existing.task_id)?;
        return Err(TimerError::ActiveSessionConflict {
            task_id: existing.task_id,
            task_title,
            class_course_code,
        });
    }

    conn.execute(
        "INSERT INTO time_sessions (task_id, start_ts) VALUES (?1, datetime('now'))",
        [task_id],
    )?;
    let id = conn.last_insert_rowid();
    let session = conn.query_row(
        "SELECT * FROM time_sessions WHERE id = ?1",
        [id],
        row_to_session,
    )?;
    conn.execute(
        "UPDATE tasks SET status = 'in_progress', updated_at = datetime('now')
         WHERE id = ?1 AND status = 'not_started'",
        [task_id],
    )?;
    build_active_info(conn, session).map_err(TimerError::from)
}

pub fn core_pause_timer(conn: &Connection) -> Result<ActiveSessionInfo, TimerError> {
    let session = find_active_session(conn)?.ok_or(TimerError::NotFound)?;
    if session.pause_started_ts.is_none() {
        conn.execute(
            "UPDATE time_sessions SET pause_started_ts = datetime('now') WHERE id = ?1",
            [session.id],
        )?;
    }
    let refreshed = conn.query_row(
        "SELECT * FROM time_sessions WHERE id = ?1",
        [session.id],
        row_to_session,
    )?;
    build_active_info(conn, refreshed).map_err(TimerError::from)
}

pub fn core_resume_timer(conn: &Connection) -> Result<ActiveSessionInfo, TimerError> {
    let session = find_active_session(conn)?.ok_or(TimerError::NotFound)?;
    if let Some(pause_started) = &session.pause_started_ts {
        let pause_elapsed: i64 = conn.query_row(
            "SELECT MAX(0, unixepoch('now') - unixepoch(?1))",
            [pause_started],
            |r| r.get(0),
        )?;
        conn.execute(
            "UPDATE time_sessions SET accumulated_pause_seconds = accumulated_pause_seconds + ?1,
             pause_started_ts = NULL WHERE id = ?2",
            rusqlite::params![pause_elapsed, session.id],
        )?;
    }
    let refreshed = conn.query_row(
        "SELECT * FROM time_sessions WHERE id = ?1",
        [session.id],
        row_to_session,
    )?;
    build_active_info(conn, refreshed).map_err(TimerError::from)
}

pub fn core_finish_timer(conn: &Connection) -> Result<Session, TimerError> {
    let session = find_active_session(conn)?.ok_or(TimerError::NotFound)?;

    // Fold any in-progress pause into accumulated_pause_seconds before finalizing.
    if let Some(pause_started) = &session.pause_started_ts {
        let pause_elapsed: i64 = conn.query_row(
            "SELECT MAX(0, unixepoch('now') - unixepoch(?1))",
            [pause_started],
            |r| r.get(0),
        )?;
        conn.execute(
            "UPDATE time_sessions SET accumulated_pause_seconds = accumulated_pause_seconds + ?1,
             pause_started_ts = NULL WHERE id = ?2",
            rusqlite::params![pause_elapsed, session.id],
        )?;
    }

    conn.execute(
        "UPDATE time_sessions SET end_ts = datetime('now'),
         final_duration_seconds = MAX(0, unixepoch('now') - unixepoch(start_ts) - accumulated_pause_seconds)
         WHERE id = ?1",
        [session.id],
    )?;

    conn.query_row(
        "SELECT * FROM time_sessions WHERE id = ?1",
        [session.id],
        row_to_session,
    )
    .map_err(TimerError::from)
}

pub fn core_cancel_timer(conn: &Connection) -> Result<(), TimerError> {
    let session = find_active_session(conn)?.ok_or(TimerError::NotFound)?;
    conn.execute("DELETE FROM time_sessions WHERE id = ?1", [session.id])?;
    Ok(())
}

pub fn core_edit_session_duration(
    conn: &Connection,
    session_id: i64,
    final_duration_seconds: i64,
) -> Result<Session, String> {
    if final_duration_seconds < 0 {
        return Err("duration cannot be negative".into());
    }
    let ended: bool = conn
        .query_row(
            "SELECT end_ts IS NOT NULL FROM time_sessions WHERE id=?1",
            [session_id],
            |r| r.get(0),
        )
        .map_err(|_| "Session not found")?;
    if !ended {
        return Err("Finish the session before editing its duration".into());
    }
    conn.execute(
        "UPDATE time_sessions SET final_duration_seconds = ?1, edited = 1 WHERE id = ?2",
        rusqlite::params![final_duration_seconds, session_id],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT * FROM time_sessions WHERE id = ?1",
        [session_id],
        row_to_session,
    )
    .map_err(|e| e.to_string())
}

pub fn core_switch_timer(
    conn: &Connection,
    session_id: i64,
    task_id: i64,
) -> Result<ActiveSessionInfo, TimerError> {
    let tx = conn.unchecked_transaction()?;
    if find_active_session(&tx)?.map(|s| s.id) != Some(session_id) {
        return Err(TimerError::NotFound);
    }
    core_finish_timer(&tx)?;
    let next = core_start_timer(&tx, task_id)?;
    tx.commit()?;
    Ok(next)
}

#[tauri::command]
pub fn switch_timer(
    db: State<Db>,
    session_id: i64,
    task_id: i64,
) -> Result<ActiveSessionInfo, TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_switch_timer(&conn, session_id, task_id)
}

#[tauri::command]
pub fn recover_timer(
    db: State<Db>,
    session_id: i64,
    duration_seconds: i64,
) -> Result<Session, String> {
    let conn = db.0.lock().map_err(|_| "Couldn't access the timer")?;
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| "Couldn't save the timer")?;
    if find_active_session(&tx)
        .map_err(|_| "Couldn't load the timer")?
        .map(|s| s.id)
        != Some(session_id)
    {
        return Err("The active session has changed. Refresh and try again.".into());
    }
    core_finish_timer(&tx).map_err(|_| "Couldn't finish the timer")?;
    let result = core_edit_session_duration(&tx, session_id, duration_seconds)?;
    tx.commit().map_err(|_| "Couldn't save the timer")?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Tauri command wrappers — thin glue: lock the shared connection, delegate.
// ---------------------------------------------------------------------------

fn lock_err(e: impl std::fmt::Display) -> TimerError {
    TimerError::Other {
        message: e.to_string(),
    }
}

#[tauri::command]
pub fn get_active_session(db: State<Db>) -> Result<Option<ActiveSessionInfo>, TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_get_active_session(&conn)
}

#[tauri::command]
pub fn start_timer(db: State<Db>, task_id: i64) -> Result<ActiveSessionInfo, TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_start_timer(&conn, task_id)
}

#[tauri::command]
pub fn pause_timer(db: State<Db>) -> Result<ActiveSessionInfo, TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_pause_timer(&conn)
}

#[tauri::command]
pub fn resume_timer(db: State<Db>) -> Result<ActiveSessionInfo, TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_resume_timer(&conn)
}

#[tauri::command]
pub fn finish_timer(db: State<Db>) -> Result<Session, TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_finish_timer(&conn)
}

#[tauri::command]
pub fn cancel_timer(db: State<Db>) -> Result<(), TimerError> {
    let conn = db.0.lock().map_err(lock_err)?;
    core_cancel_timer(&conn)
}

#[tauri::command]
pub fn list_sessions_for_task(db: State<Db>, task_id: i64) -> Result<Vec<Session>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT * FROM time_sessions WHERE task_id = ?1 ORDER BY start_ts")
        .map_err(|e| e.to_string())?;
    let result = stmt
        .query_map([task_id], row_to_session)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}

/// Manual correction (Section 12): overrides the authoritative duration while
/// preserving the original timestamps for audit.
#[tauri::command]
pub fn edit_session_duration(
    db: State<Db>,
    session_id: i64,
    final_duration_seconds: i64,
) -> Result<Session, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    core_edit_session_duration(&conn, session_id, final_duration_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::thread::sleep;
    use std::time::Duration;

    fn test_conn() -> Connection {
        db::test_connection()
    }

    fn seed_task(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO classes (course_code, semester) VALUES ('TEST 1000', 'Fall 2026')",
            [],
        )
        .unwrap();
        let class_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO tasks (class_id, title) VALUES (?1, 'Test task')",
            [class_id],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn start_then_finish_produces_positive_duration() {
        let conn = test_conn();
        let task_id = seed_task(&conn);

        core_start_timer(&conn, task_id).expect("start");
        sleep(Duration::from_millis(1100));
        let finished = core_finish_timer(&conn).expect("finish");

        assert!(finished.end_ts.is_some());
        assert!(finished.final_duration_seconds.unwrap() >= 1);
    }

    #[test]
    fn second_start_is_rejected_with_conflict() {
        let conn = test_conn();
        let task_a = seed_task(&conn);
        let task_b = seed_task(&conn);

        core_start_timer(&conn, task_a).expect("first start");
        let err = core_start_timer(&conn, task_b).unwrap_err();

        match err {
            TimerError::ActiveSessionConflict { task_id, .. } => assert_eq!(task_id, task_a),
            other => panic!("expected ActiveSessionConflict, got {other:?}"),
        }
    }

    #[test]
    fn pause_then_resume_excludes_paused_time_from_duration() {
        // Uses a long pause relative to the active windows and a loose
        // tolerance band, so scheduling jitter under parallel test execution
        // can't flip the result — only an actual "pause didn't exclude time"
        // regression would push final_duration_seconds anywhere near total_wall.
        let conn = test_conn();
        let task_id = seed_task(&conn);
        let wall_clock_start = std::time::Instant::now();

        core_start_timer(&conn, task_id).expect("start");
        sleep(Duration::from_millis(600));
        let paused = core_pause_timer(&conn).expect("pause");
        assert!(paused.is_paused);
        sleep(Duration::from_secs(3));
        let resumed = core_resume_timer(&conn).expect("resume");
        assert!(!resumed.is_paused);
        assert!(resumed.session.accumulated_pause_seconds >= 2);
        sleep(Duration::from_millis(600));
        let finished = core_finish_timer(&conn).expect("finish");

        let total_wall_seconds = wall_clock_start.elapsed().as_secs_f64();
        let duration = finished.final_duration_seconds.unwrap();

        // Active time (~1.2s) should be tracked; the ~3s pause should not be.
        assert!(
            duration <= 2,
            "expected paused time excluded, got {duration}s"
        );
        assert!(
            (duration as f64) < total_wall_seconds - 1.5,
            "duration ({duration}s) should be well under total wall time ({total_wall_seconds:.1}s) \
             if the pause was actually excluded"
        );
    }

    #[test]
    fn multiple_pauses_accumulate() {
        let conn = test_conn();
        let task_id = seed_task(&conn);

        core_start_timer(&conn, task_id).expect("start");
        core_pause_timer(&conn).expect("pause 1");
        sleep(Duration::from_millis(600));
        core_resume_timer(&conn).expect("resume 1");
        core_pause_timer(&conn).expect("pause 2");
        sleep(Duration::from_millis(600));
        let resumed = core_resume_timer(&conn).expect("resume 2");

        assert!(resumed.session.accumulated_pause_seconds >= 1);
    }

    #[test]
    fn cancel_deletes_the_session_without_counting_it() {
        let conn = test_conn();
        let task_id = seed_task(&conn);

        core_start_timer(&conn, task_id).expect("start");
        core_cancel_timer(&conn).expect("cancel");

        let active = core_get_active_session(&conn).expect("get active");
        assert!(active.is_none());

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM time_sessions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn finish_with_no_active_session_returns_not_found() {
        let conn = test_conn();
        let err = core_finish_timer(&conn).unwrap_err();
        assert_eq!(err, TimerError::NotFound);
    }

    #[test]
    fn restart_recovery_recomputes_elapsed_from_timestamps_not_a_counter() {
        // Simulates "app crashes mid-session, reopened later": the active
        // session survives as a DB row, and elapsed time is derived purely
        // from stored timestamps — never from any in-memory counter that a
        // crash would have destroyed.
        let conn = test_conn();
        let task_id = seed_task(&conn);
        core_start_timer(&conn, task_id).expect("start");

        // Backdate start_ts to simulate time having passed while "crashed".
        conn.execute(
            "UPDATE time_sessions SET start_ts = datetime('now', '-90 seconds')
             WHERE task_id = ?1",
            [task_id],
        )
        .unwrap();

        let recovered = core_get_active_session(&conn)
            .expect("get active")
            .expect("session should still be active");
        assert!(recovered.elapsed_seconds >= 89 && recovered.elapsed_seconds <= 91);
    }

    #[test]
    fn manual_edit_overrides_duration_and_marks_edited() {
        let conn = test_conn();
        let task_id = seed_task(&conn);
        core_start_timer(&conn, task_id).expect("start");
        sleep(Duration::from_millis(1100));
        let finished = core_finish_timer(&conn).expect("finish");

        let edited = core_edit_session_duration(&conn, finished.id, 3420).expect("edit");
        assert_eq!(edited.final_duration_seconds, Some(3420));
        assert!(edited.edited);
    }

    #[test]
    fn manual_edit_rejects_negative_duration() {
        let conn = test_conn();
        let task_id = seed_task(&conn);
        core_start_timer(&conn, task_id).expect("start");
        let finished = core_finish_timer(&conn).expect("finish");

        let err = core_edit_session_duration(&conn, finished.id, -5).unwrap_err();
        assert!(err.contains("negative"));
    }

    #[test]
    fn day_boundary_crossing_does_not_break_elapsed_computation() {
        // A session whose start_ts is "yesterday" per wall-clock date must
        // still compute a correct elapsed duration — julianday() arithmetic
        // is date-agnostic, so crossing midnight (or a DST boundary, which
        // SQLite's julianday also does not adjust for local wall-clock jumps)
        // must not corrupt the result.
        let conn = test_conn();
        let task_id = seed_task(&conn);
        core_start_timer(&conn, task_id).expect("start");
        conn.execute(
            "UPDATE time_sessions SET start_ts = datetime('now', '-1 day', '-30 seconds')
             WHERE task_id = ?1",
            [task_id],
        )
        .unwrap();

        let active = core_get_active_session(&conn)
            .expect("get active")
            .expect("still active");
        let expected = 24 * 3600 + 30;
        assert!(
            (active.elapsed_seconds - expected).abs() <= 2,
            "expected ~{expected}s, got {}s",
            active.elapsed_seconds
        );
    }
    #[test]
    fn database_rejects_second_active_session_without_app_checks() {
        let conn = test_conn();
        let id = seed_task(&conn);
        conn.execute(
            "INSERT INTO time_sessions(task_id,start_ts) VALUES(?1,datetime('now'))",
            [id],
        )
        .unwrap();
        assert!(conn
            .execute(
                "INSERT INTO time_sessions(task_id,start_ts) VALUES(?1,datetime('now'))",
                [id]
            )
            .is_err());
    }
    #[test]
    fn switching_is_atomic_and_preserves_requested_task() {
        let conn = test_conn();
        let a = seed_task(&conn);
        let b = seed_task(&conn);
        let first = core_start_timer(&conn, a).unwrap();
        assert!(core_switch_timer(&conn, first.session.id, 999999).is_err());
        assert_eq!(
            core_get_active_session(&conn).unwrap().unwrap().session.id,
            first.session.id
        );
        let next = core_switch_timer(&conn, first.session.id, b).unwrap();
        assert_eq!(next.session.task_id, b);
        assert!(core_switch_timer(&conn, first.session.id, a).is_err());
    }
    #[test]
    fn sleep_gap_and_paused_gap_are_timestamp_derived() {
        let conn = test_conn();
        let id = seed_task(&conn);
        core_start_timer(&conn, id).unwrap();
        conn.execute_batch("UPDATE time_sessions SET start_ts=datetime('now','-7200 seconds'), pause_started_ts=datetime('now','-3600 seconds'), accumulated_pause_seconds=600").unwrap();
        assert_eq!(
            core_get_active_session(&conn)
                .unwrap()
                .unwrap()
                .elapsed_seconds,
            3000
        );
        assert_eq!(
            core_finish_timer(&conn).unwrap().final_duration_seconds,
            Some(3000)
        );
    }
    #[test]
    fn cannot_edit_a_running_session() {
        let conn = test_conn();
        let id = seed_task(&conn);
        let active = core_start_timer(&conn, id).unwrap();
        assert!(core_edit_session_duration(&conn, active.session.id, 60).is_err());
    }
}
