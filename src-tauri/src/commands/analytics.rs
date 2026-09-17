use crate::db::Db;
use chrono::{Duration, NaiveDate};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct EstimateBreakdown {
    pub key: String,
    pub label: String,
    pub sample_count: i64,
    pub median_estimated_minutes: i64,
    pub median_actual_minutes: i64,
    pub median_error_percent: i64,
}

#[derive(Debug, Serialize)]
pub struct EstimateBand {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub count: i64,
    pub median_seconds: i64,
    pub average_seconds: i64,
    pub longest_seconds: i64,
    pub under_ten_minutes: i64,
    pub at_least_thirty_minutes: i64,
}

#[derive(Debug, Serialize)]
pub struct BlockActualSummary {
    pub planned_minutes: i64,
    pub completed_planned_minutes: i64,
    pub actual_tracked_seconds: i64,
    pub block_count: i64,
    pub completed_block_count: i64,
}

#[derive(Debug, Serialize)]
pub struct EstimateAnalytics {
    pub sample_count: i64,
    pub median_estimated_minutes: i64,
    pub median_actual_minutes: i64,
    pub median_difference_minutes: i64,
    pub median_error_percent: i64,
    pub bands: Vec<EstimateBand>,
    pub by_class: Vec<EstimateBreakdown>,
    pub by_task_type: Vec<EstimateBreakdown>,
    pub session_stats: SessionStats,
    pub blocks: BlockActualSummary,
}

#[derive(Debug, Serialize)]
pub struct EstimateSuggestion {
    pub suggested_minutes: Option<i64>,
    pub sample_count: i64,
    pub source: Option<String>,
    pub median_actual_minutes: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct EstimateMiss {
    pub task_id: i64,
    pub task_title: String,
    pub course_code: String,
    pub estimated_minutes: i64,
    pub actual_minutes: i64,
    pub difference_minutes: i64,
    pub error_percent: i64,
}

#[derive(Debug, Serialize)]
pub struct WorkloadDay {
    pub date: String,
    pub available_minutes: Option<i64>,
    pub scheduled_block_minutes: i64,
    pub remaining_due_minutes: i64,
    pub remaining_capacity_minutes: Option<i64>,
    pub overflow_minutes: i64,
    pub load_percent: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WeeklyReview {
    pub start_date: String,
    pub end_date: String,
    pub tracked_seconds: i64,
    pub completed_tasks: i64,
    pub overdue_tasks: i64,
    pub largest_estimate_miss: Option<EstimateMiss>,
    pub most_time_class: Option<ClassTotal>,
    pub median_session_seconds: i64,
    pub schedule_coverage_percent: i64,
    pub blocks: BlockActualSummary,
    pub workload: Vec<WorkloadDay>,
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

/// Date-only due dates (10 chars) are floating local dates; timestamp deadlines
/// are converted via 'localtime'. `{0}` is substituted with the due-date column
/// reference (e.g. `t.due_at` or `due_at`) — matches the pattern in
/// today.rs/week.rs/planner.rs. Bare `date(due_at)` (without this normalization)
/// misdates timestamp due values by treating them as UTC.
fn due_date_expr(column: &str) -> String {
    format!("CASE WHEN length({column})=10 THEN date({column}) ELSE date({column},'localtime') END")
}

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

#[derive(Debug)]
struct EstimateSample {
    task_id: i64,
    task_title: String,
    course_id: i64,
    course_code: String,
    task_type: String,
    estimated_minutes: i64,
    actual_minutes: i64,
}

fn median(values: &mut [i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

fn error_percent(actual: i64, estimated: i64) -> i64 {
    if estimated <= 0 {
        0
    } else {
        ((actual - estimated) * 100) / estimated
    }
}

fn estimate_samples(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<Vec<EstimateSample>> {
    let sql = format!(
        "SELECT t.id, t.title, c.id, c.course_code, t.task_type, t.estimated_minutes,
                COALESCE(SUM({DURATION_EXPR}), 0)
         FROM tasks t
         JOIN classes c ON c.id=t.class_id
         LEFT JOIN time_sessions ts ON ts.task_id=t.id AND ts.end_ts IS NOT NULL
         WHERE t.status='completed' AND t.estimated_minutes IS NOT NULL
           AND t.completed_at IS NOT NULL
           AND date(t.completed_at,'localtime') BETWEEN date(?1) AND date(?2)
         GROUP BY t.id, t.title, c.id, c.course_code, t.task_type, t.estimated_minutes
         ORDER BY t.completed_at, t.id"
    );
    conn.prepare(&sql)?
        .query_map(rusqlite::params![start_date, end_date], |row| {
            Ok(EstimateSample {
                task_id: row.get(0)?,
                task_title: row.get(1)?,
                course_id: row.get(2)?,
                course_code: row.get(3)?,
                task_type: row.get(4)?,
                estimated_minutes: row.get(5)?,
                actual_minutes: row.get::<_, i64>(6)? / 60,
            })
        })?
        .collect()
}

fn breakdowns(samples: &[EstimateSample], by_class: bool) -> Vec<EstimateBreakdown> {
    use std::collections::BTreeMap;
    let mut grouped: BTreeMap<String, Vec<&EstimateSample>> = BTreeMap::new();
    for sample in samples {
        let key = if by_class {
            sample.course_id.to_string()
        } else {
            sample.task_type.clone()
        };
        grouped.entry(key).or_default().push(sample);
    }
    grouped
        .into_iter()
        .map(|(key, values)| {
            let mut estimates: Vec<i64> = values.iter().map(|s| s.estimated_minutes).collect();
            let mut actuals: Vec<i64> = values.iter().map(|s| s.actual_minutes).collect();
            let mut errors: Vec<i64> = values
                .iter()
                .map(|s| error_percent(s.actual_minutes, s.estimated_minutes))
                .collect();
            let label = if by_class {
                values[0].course_code.clone()
            } else {
                key.replace('_', " ")
            };
            EstimateBreakdown {
                key,
                label,
                sample_count: values.len() as i64,
                median_estimated_minutes: median(&mut estimates),
                median_actual_minutes: median(&mut actuals),
                median_error_percent: median(&mut errors),
            }
        })
        .collect()
}

fn session_stats(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<SessionStats> {
    let sql = format!(
        "SELECT {DURATION_EXPR}
         FROM time_sessions ts
         WHERE ts.end_ts IS NOT NULL
           AND date(ts.start_ts,'localtime') BETWEEN date(?1) AND date(?2)"
    );
    let mut durations: Vec<i64> = conn
        .prepare(&sql)?
        .query_map(rusqlite::params![start_date, end_date], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    let count = durations.len() as i64;
    let total: i64 = durations.iter().sum();
    let longest_seconds = durations.iter().copied().max().unwrap_or(0);
    let under_ten_minutes = durations.iter().filter(|seconds| **seconds < 600).count() as i64;
    let at_least_thirty_minutes =
        durations.iter().filter(|seconds| **seconds >= 1800).count() as i64;
    let median_seconds = median(&mut durations);
    Ok(SessionStats {
        count,
        median_seconds,
        average_seconds: if count > 0 { total / count } else { 0 },
        longest_seconds,
        under_ten_minutes,
        at_least_thirty_minutes,
    })
}

fn block_summary(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<BlockActualSummary> {
    let (planned_minutes, completed_planned_minutes, block_count, completed_block_count): (
        i64,
        i64,
        i64,
        i64,
    ) = conn.query_row(
        "SELECT COALESCE(SUM(planned_minutes),0),
                COALESCE(SUM(CASE WHEN completed=1 THEN planned_minutes ELSE 0 END),0),
                COUNT(*), COALESCE(SUM(completed),0)
         FROM study_blocks WHERE planned_date BETWEEN ?1 AND ?2",
        rusqlite::params![start_date, end_date],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let sql = format!(
        "SELECT COALESCE(SUM({DURATION_EXPR}),0) FROM time_sessions ts
         WHERE ts.end_ts IS NOT NULL AND date(ts.start_ts,'localtime') BETWEEN date(?1) AND date(?2)
           AND EXISTS (SELECT 1 FROM study_blocks sb WHERE sb.task_id=ts.task_id AND sb.planned_date BETWEEN ?1 AND ?2)"
    );
    let actual_tracked_seconds =
        conn.query_row(&sql, rusqlite::params![start_date, end_date], |row| {
            row.get(0)
        })?;
    Ok(BlockActualSummary {
        planned_minutes,
        completed_planned_minutes,
        actual_tracked_seconds,
        block_count,
        completed_block_count,
    })
}

#[tauri::command]
pub fn get_estimate_analytics(
    db: State<Db>,
    start_date: String,
    end_date: String,
) -> Result<EstimateAnalytics, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let samples = estimate_samples(&conn, &start_date, &end_date).map_err(|e| e.to_string())?;
    let mut estimates: Vec<i64> = samples.iter().map(|s| s.estimated_minutes).collect();
    let mut actuals: Vec<i64> = samples.iter().map(|s| s.actual_minutes).collect();
    let mut differences: Vec<i64> = samples
        .iter()
        .map(|s| s.actual_minutes - s.estimated_minutes)
        .collect();
    let mut errors: Vec<i64> = samples
        .iter()
        .map(|s| error_percent(s.actual_minutes, s.estimated_minutes))
        .collect();
    let bands = [
        (
            "Within estimate",
            samples
                .iter()
                .filter(|s| s.actual_minutes <= s.estimated_minutes)
                .count(),
        ),
        (
            "0–25% over",
            samples
                .iter()
                .filter(|s| {
                    s.actual_minutes > s.estimated_minutes
                        && error_percent(s.actual_minutes, s.estimated_minutes) <= 25
                })
                .count(),
        ),
        (
            "25–50% over",
            samples
                .iter()
                .filter(|s| {
                    error_percent(s.actual_minutes, s.estimated_minutes) > 25
                        && error_percent(s.actual_minutes, s.estimated_minutes) <= 50
                })
                .count(),
        ),
        (
            ">50% over",
            samples
                .iter()
                .filter(|s| error_percent(s.actual_minutes, s.estimated_minutes) > 50)
                .count(),
        ),
    ]
    .into_iter()
    .map(|(label, count)| EstimateBand {
        label: label.into(),
        count: count as i64,
    })
    .collect();
    Ok(EstimateAnalytics {
        sample_count: samples.len() as i64,
        median_estimated_minutes: median(&mut estimates),
        median_actual_minutes: median(&mut actuals),
        median_difference_minutes: median(&mut differences),
        median_error_percent: median(&mut errors),
        bands,
        by_class: breakdowns(&samples, true),
        by_task_type: breakdowns(&samples, false),
        session_stats: session_stats(&conn, &start_date, &end_date).map_err(|e| e.to_string())?,
        blocks: block_summary(&conn, &start_date, &end_date).map_err(|e| e.to_string())?,
    })
}

fn suggestion_for(
    samples: &[EstimateSample],
    class_id: Option<i64>,
    task_type: Option<&str>,
) -> Option<(i64, i64)> {
    let filtered: Vec<&EstimateSample> = samples
        .iter()
        .filter(|sample| {
            class_id.is_none_or(|id| sample.course_id == id)
                && task_type.is_none_or(|kind| sample.task_type == kind)
        })
        .collect();
    if filtered.len() < 3 {
        return None;
    }
    let mut actuals: Vec<i64> = filtered.iter().map(|s| s.actual_minutes).collect();
    Some((median(&mut actuals), filtered.len() as i64))
}

#[tauri::command]
pub fn get_estimate_suggestion(
    db: State<Db>,
    class_id: i64,
    task_type: String,
) -> Result<EstimateSuggestion, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let samples = estimate_samples(&conn, "1900-01-01", "2999-12-31").map_err(|e| e.to_string())?;
    let candidates = [
        (Some(class_id), Some(task_type.as_str()), "class_and_type"),
        (None, Some(task_type.as_str()), "task_type"),
        (Some(class_id), None, "class"),
        (None, None, "global"),
    ];
    for (class_filter, type_filter, source) in candidates {
        if let Some((minutes, count)) = suggestion_for(&samples, class_filter, type_filter) {
            return Ok(EstimateSuggestion {
                suggested_minutes: Some(minutes),
                sample_count: count,
                source: Some(source.into()),
                median_actual_minutes: Some(minutes),
            });
        }
    }
    Ok(EstimateSuggestion {
        suggested_minutes: None,
        sample_count: samples.len() as i64,
        source: None,
        median_actual_minutes: None,
    })
}

fn capacity_for_date(conn: &rusqlite::Connection, date: &str) -> rusqlite::Result<Option<i64>> {
    if let Ok(minutes) = conn.query_row(
        "SELECT available_minutes FROM study_capacity_overrides WHERE date=?1",
        [date],
        |row| row.get::<_, i64>(0),
    ) {
        return Ok(Some(minutes));
    }
    let weekday = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|value| value.format("%u").to_string())
        .unwrap_or_else(|_| "1".into());
    match conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key=?1",
        [format!("study_capacity_{weekday}")],
        |row| row.get::<_, i64>(0),
    ) {
        Ok(minutes) => Ok(Some(minutes)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

fn workload_days(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<Vec<WorkloadDay>> {
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d")
        .map_err(|_| rusqlite::Error::InvalidQuery)?;
    let mut days = Vec::new();
    let mut date = start;
    while date <= end {
        let date_string = date.to_string();
        let scheduled_block_minutes: i64 = conn.query_row(
            "SELECT COALESCE(SUM(planned_minutes),0) FROM study_blocks WHERE planned_date=?1 AND completed=0",
            [&date_string], |row| row.get::<_, i64>(0)
        )?;
        let due_expr = due_date_expr("t.due_at");
        let sql = format!(
            "SELECT t.estimated_minutes, COALESCE(SUM({DURATION_EXPR}),0),
                    COALESCE((SELECT SUM(sb.planned_minutes) FROM study_blocks sb
                              WHERE sb.task_id=t.id AND sb.planned_date <= substr(t.due_at,1,10)),0)
             FROM tasks t
             LEFT JOIN time_sessions ts ON ts.task_id=t.id AND ts.end_ts IS NOT NULL
             WHERE t.status!='completed' AND t.estimated_minutes IS NOT NULL
               AND t.due_at IS NOT NULL AND {due_expr}=date(?1)
             GROUP BY t.id, t.estimated_minutes, t.due_at"
        );
        let mut stmt = conn.prepare(&sql)?;
        let remaining_due_minutes: i64 = stmt
            .query_map([&date_string], |row| {
                let estimate: i64 = row.get(0)?;
                let actual_minutes: i64 = row.get::<_, i64>(1)? / 60;
                let blocks: i64 = row.get(2)?;
                Ok((estimate.saturating_sub(actual_minutes).max(0) - blocks).max(0))
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .sum();
        let available_minutes = capacity_for_date(conn, &date_string)?;
        let remaining_capacity_minutes =
            available_minutes.map(|capacity| capacity - scheduled_block_minutes);
        let overflow_minutes: i64 = match available_minutes {
            Some(capacity) => (scheduled_block_minutes + remaining_due_minutes - capacity).max(0),
            None => 0,
        };
        let load_percent = available_minutes
            .filter(|capacity| *capacity > 0)
            .map(|capacity| (scheduled_block_minutes + remaining_due_minutes) * 100 / capacity);
        days.push(WorkloadDay {
            date: date_string,
            available_minutes,
            scheduled_block_minutes,
            remaining_due_minutes,
            remaining_capacity_minutes,
            overflow_minutes,
            load_percent,
        });
        date += Duration::days(1);
    }
    Ok(days)
}

fn schedule_coverage(
    conn: &rusqlite::Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<i64> {
    let due_expr = due_date_expr("t.due_at");
    let sql = format!(
        "SELECT COALESCE(SUM(t.estimated_minutes - CAST(COALESCE(SUM({DURATION_EXPR}),0) AS INTEGER)/60),0),
                COALESCE(SUM((SELECT SUM(sb.planned_minutes) FROM study_blocks sb
                              WHERE sb.task_id=t.id AND sb.planned_date <= substr(t.due_at,1,10))),0)
         FROM tasks t
         LEFT JOIN time_sessions ts ON ts.task_id=t.id AND ts.end_ts IS NOT NULL
         WHERE t.status!='completed' AND t.estimated_minutes IS NOT NULL AND t.due_at IS NOT NULL
           AND {due_expr} BETWEEN date(?1) AND date(?2)
         GROUP BY t.id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut remaining = 0i64;
    let mut planned = 0i64;
    for row in stmt.query_map(rusqlite::params![start_date, end_date], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })? {
        let (row_remaining, row_planned) = row?;
        remaining += row_remaining.max(0);
        planned += row_planned;
    }
    Ok(if remaining > 0 {
        (planned * 100 / remaining).min(100)
    } else {
        100
    })
}

#[tauri::command]
pub fn get_weekly_review(db: State<Db>, start_date: String) -> Result<WeeklyReview, String> {
    let start = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")
        .map_err(|_| "Choose a valid start date.".to_string())?;
    let end_date = (start + Duration::days(6)).to_string();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let summary_sql = format!(
        "SELECT c.id, c.course_code, COALESCE(SUM({DURATION_EXPR}),0)
         FROM time_sessions ts JOIN tasks t ON t.id=ts.task_id JOIN classes c ON c.id=t.class_id
         WHERE ts.end_ts IS NOT NULL AND date(ts.start_ts,'localtime') BETWEEN date(?1) AND date(?2)
         GROUP BY c.id,c.course_code ORDER BY 3 DESC"
    );
    let by_class: Vec<ClassTotal> = conn
        .prepare(&summary_sql)
        .map_err(|e| e.to_string())?
        .query_map(rusqlite::params![start_date, end_date], |row| {
            Ok(ClassTotal {
                class_id: row.get(0)?,
                course_code: row.get(1)?,
                tracked_seconds: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let tracked_seconds_total = by_class.iter().map(|item| item.tracked_seconds).sum();
    let completed_tasks: i64 = conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE status='completed' AND completed_at IS NOT NULL AND date(completed_at,'localtime') BETWEEN date(?1) AND date(?2)",
        rusqlite::params![start_date, end_date], |row| row.get(0)
    ).map_err(|e| e.to_string())?;
    let overdue_tasks: i64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM tasks WHERE status!='completed' AND due_at IS NOT NULL AND {} < date(?1)",
                due_date_expr("due_at")
            ),
            [&end_date],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let samples = estimate_samples(&conn, &start_date, &end_date).map_err(|e| e.to_string())?;
    let largest_estimate_miss = samples
        .iter()
        .max_by_key(|sample| (sample.actual_minutes - sample.estimated_minutes).abs())
        .map(|sample| EstimateMiss {
            task_id: sample.task_id,
            task_title: sample.task_title.clone(),
            course_code: sample.course_code.clone(),
            estimated_minutes: sample.estimated_minutes,
            actual_minutes: sample.actual_minutes,
            difference_minutes: sample.actual_minutes - sample.estimated_minutes,
            error_percent: error_percent(sample.actual_minutes, sample.estimated_minutes),
        });
    let most_time_class = by_class.first().cloned();
    let stats = session_stats(&conn, &start_date, &end_date).map_err(|e| e.to_string())?;
    Ok(WeeklyReview {
        start_date: start_date.clone(),
        end_date: end_date.clone(),
        tracked_seconds: tracked_seconds_total,
        completed_tasks,
        overdue_tasks,
        largest_estimate_miss,
        most_time_class,
        median_session_seconds: stats.median_seconds,
        schedule_coverage_percent: schedule_coverage(&conn, &start_date, &end_date)
            .map_err(|e| e.to_string())?,
        blocks: block_summary(&conn, &start_date, &end_date).map_err(|e| e.to_string())?,
        workload: workload_days(&conn, &start_date, &end_date).map_err(|e| e.to_string())?,
    })
}
