use crate::db::Db;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ClassTotal {
    pub class_id: i64,
    pub course_code: String,
    pub tracked_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct RangeSummary {
    pub tracked_seconds_total: i64,
    pub by_class: Vec<ClassTotal>,
}

#[derive(Debug, Serialize)]
pub struct SessionEntry {
    pub session_id: i64,
    pub task_id: i64,
    pub task_title: String,
    pub course_code: String,
    pub start_ts: String,
    pub end_ts: Option<String>,
    pub duration_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct DayView {
    pub date: String,
    pub total_seconds: i64,
    pub by_class: Vec<ClassTotal>,
    pub sessions: Vec<SessionEntry>,
}

#[derive(Debug, Serialize)]
pub struct TaskHistory {
    pub task_id: i64,
    pub total_seconds: i64,
    pub estimated_minutes: Option<i64>,
    pub sessions: Vec<SessionEntry>,
}

/// duration = final_duration_seconds if set, else computed from timestamps
/// for a still-open (crashed/uncommitted) row — never double counted since
/// each finished session has exactly one row.
const DURATION_EXPR: &str = "
    COALESCE(ts.final_duration_seconds,
        CASE WHEN ts.end_ts IS NOT NULL
            THEN CAST((julianday(ts.end_ts) - julianday(ts.start_ts)) * 86400 AS INTEGER)
                 - ts.accumulated_pause_seconds
            ELSE 0 END)
";

fn range_summary(
    conn: &rusqlite::Connection,
    date_filter_sql: &str,
) -> rusqlite::Result<RangeSummary> {
    let sql = format!(
        "SELECT c.id, c.course_code, COALESCE(SUM({DURATION_EXPR}), 0) as secs
         FROM time_sessions ts
         JOIN tasks t ON t.id = ts.task_id
         JOIN classes c ON c.id = t.class_id
         WHERE ts.end_ts IS NOT NULL AND {date_filter_sql}
         GROUP BY c.id, c.course_code
         ORDER BY secs DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let by_class: Vec<ClassTotal> = stmt
        .query_map([], |row| {
            Ok(ClassTotal {
                class_id: row.get(0)?,
                course_code: row.get(1)?,
                tracked_seconds: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let tracked_seconds_total = by_class.iter().map(|c| c.tracked_seconds).sum();
    Ok(RangeSummary {
        tracked_seconds_total,
        by_class,
    })
}

#[tauri::command]
pub fn get_analytics_today(db: State<Db>) -> Result<RangeSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    range_summary(
        &conn,
        "date(ts.start_ts, 'localtime') = date('now', 'localtime')",
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_analytics_week(db: State<Db>) -> Result<RangeSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    range_summary(
        &conn,
        "date(ts.start_ts, 'localtime') >= date('now', 'localtime', '-' || ((CAST(strftime('%w', 'now', 'localtime') AS INTEGER) + 6) % 7) || ' days')",
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_analytics_month(db: State<Db>) -> Result<RangeSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    range_summary(
        &conn,
        "strftime('%Y-%m', ts.start_ts, 'localtime') = strftime('%Y-%m', 'now', 'localtime')",
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_day_view(db: State<Db>, date: String) -> Result<DayView, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;

    let by_class_sql = format!(
        "SELECT c.id, c.course_code, COALESCE(SUM({DURATION_EXPR}), 0) as secs
         FROM time_sessions ts
         JOIN tasks t ON t.id = ts.task_id
         JOIN classes c ON c.id = t.class_id
         WHERE ts.end_ts IS NOT NULL AND date(ts.start_ts, 'localtime') = date(?1)
         GROUP BY c.id, c.course_code
         ORDER BY secs DESC"
    );
    let mut by_class_stmt = conn.prepare(&by_class_sql).map_err(|e| e.to_string())?;
    let by_class: Vec<ClassTotal> = by_class_stmt
        .query_map([&date], |row| {
            Ok(ClassTotal {
                class_id: row.get(0)?,
                course_code: row.get(1)?,
                tracked_seconds: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let total_seconds = by_class.iter().map(|c| c.tracked_seconds).sum();

    let sql = format!(
        "SELECT ts.id, ts.task_id, t.title, c.course_code, ts.start_ts, ts.end_ts,
                {DURATION_EXPR} as secs
         FROM time_sessions ts
         JOIN tasks t ON t.id = ts.task_id
         JOIN classes c ON c.id = t.class_id
         WHERE ts.end_ts IS NOT NULL AND date(ts.start_ts, 'localtime') = date(?1)
         ORDER BY ts.start_ts"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let sessions: Vec<SessionEntry> = stmt
        .query_map([&date], |row| {
            Ok(SessionEntry {
                session_id: row.get(0)?,
                task_id: row.get(1)?,
                task_title: row.get(2)?,
                course_code: row.get(3)?,
                start_ts: row.get(4)?,
                end_ts: row.get(5)?,
                duration_seconds: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(DayView {
        date,
        total_seconds,
        by_class,
        sessions,
    })
}

#[tauri::command]
pub fn get_task_history(db: State<Db>, task_id: i64) -> Result<TaskHistory, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let estimated_minutes: Option<i64> = conn
        .query_row(
            "SELECT estimated_minutes FROM tasks WHERE id = ?1",
            [task_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let sql = format!(
        "SELECT ts.id, ts.task_id, t.title, c.course_code, ts.start_ts, ts.end_ts,
                {DURATION_EXPR} as secs
         FROM time_sessions ts
         JOIN tasks t ON t.id = ts.task_id
         JOIN classes c ON c.id = t.class_id
         WHERE ts.end_ts IS NOT NULL AND ts.task_id = ?1
         ORDER BY ts.start_ts"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let sessions: Vec<SessionEntry> = stmt
        .query_map([task_id], |row| {
            Ok(SessionEntry {
                session_id: row.get(0)?,
                task_id: row.get(1)?,
                task_title: row.get(2)?,
                course_code: row.get(3)?,
                start_ts: row.get(4)?,
                end_ts: row.get(5)?,
                duration_seconds: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let total_seconds = sessions.iter().map(|s| s.duration_seconds).sum();

    Ok(TaskHistory {
        task_id,
        total_seconds,
        estimated_minutes,
        sessions,
    })
}
