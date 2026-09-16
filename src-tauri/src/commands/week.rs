use super::tasks::{row_to_task, Task, ELIGIBLE_TASK_CLAUSE, TASK_SELECT};
use crate::db::Db;
use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct WeekTask {
    pub id: i64,
    pub class_id: i64,
    pub course_code: String,
    pub title: String,
    pub status: String,
    pub due_at: Option<String>,
    pub scheduled_date: Option<String>,
    pub estimated_minutes: Option<i64>,
    pub tracked_seconds_direct: i64,
    pub tracked_seconds_total: i64,
    pub remaining_minutes: Option<i64>,
    pub overdue: bool,
    pub context_only: bool,
}

#[derive(Debug, Serialize)]
pub struct WeekDay {
    pub date: String,
    pub tasks: Vec<WeekTask>,
    pub estimated_minutes_remaining: i64,
    pub tracked_seconds: i64,
    pub capacity_minutes: Option<i64>,
    pub load_percent: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct WeekSummary {
    pub start_date: String,
    pub end_date: String,
    pub days: Vec<WeekDay>,
    pub unscheduled: Vec<WeekTask>,
    pub estimated_minutes_remaining: i64,
    pub tracked_seconds: i64,
}

fn to_week_task(
    conn: &Connection,
    task: Task,
    today: &str,
    context_only: bool,
) -> rusqlite::Result<WeekTask> {
    let course_code: String = conn.query_row(
        "SELECT course_code FROM classes WHERE id=?1",
        [task.class_id],
        |r| r.get(0),
    )?;
    let overdue = match &task.due_at {
        Some(due) => conn.query_row(
            "SELECT CASE WHEN length(?1)=10 THEN date(?1) ELSE date(?1,'localtime') END < ?2",
            rusqlite::params![due, today],
            |r| r.get::<_, bool>(0),
        )?,
        None => false,
    };
    let not_completed = task.status != "completed";
    Ok(WeekTask {
        id: task.id,
        class_id: task.class_id,
        course_code,
        title: task.title,
        status: task.status,
        due_at: task.due_at,
        scheduled_date: task.scheduled_date,
        estimated_minutes: task.estimated_minutes,
        tracked_seconds_direct: task.tracked_seconds_direct,
        tracked_seconds_total: task.tracked_seconds,
        remaining_minutes: task.remaining_minutes,
        overdue: overdue && not_completed,
        context_only,
    })
}

fn eligible_clause() -> &'static str {
    ELIGIBLE_TASK_CLAUSE
}

fn capacity_for(conn: &Connection, date: &str, weekday: u32) -> rusqlite::Result<Option<i64>> {
    let override_minutes: Option<i64> = conn
        .query_row(
            "SELECT available_minutes FROM study_capacity_overrides WHERE date=?1",
            [date],
            |r| r.get(0),
        )
        .ok();
    if override_minutes.is_some() {
        return Ok(override_minutes);
    }
    let key = format!("study_capacity_{weekday}");
    conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key=?1",
        [key],
        |r| r.get(0),
    )
    .ok()
    .map(Ok)
    .transpose()
}

pub(crate) fn week_for(conn: &Connection, start_date: &str) -> rusqlite::Result<WeekSummary> {
    crate::commands::recurrence::ensure_generated(conn, 45)?;
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    let end = start + Duration::days(6);
    let today = chrono::Local::now().date_naive().to_string();

    let eligible = eligible_clause();
    let tasks: Vec<Task> = conn
        .prepare(&format!(
            "{TASK_SELECT} WHERE {eligible} AND t.status != 'completed' ORDER BY t.id"
        ))?
        .query_map([], row_to_task)?
        .collect::<Result<_, _>>()?;

    let by_id: std::collections::HashMap<i64, Task> =
        tasks.iter().cloned().map(|t| (t.id, t)).collect();
    let mut selected = std::collections::HashSet::new();
    for task in &tasks {
        if task
            .scheduled_date
            .as_deref()
            .is_some_and(|d| d >= start.to_string().as_str() && d <= end.to_string().as_str())
        {
            selected.insert(task.id);
            let mut parent = task.parent_task_id;
            while let Some(pid) = parent {
                let Some(parent_task) = by_id.get(&pid) else {
                    break;
                };
                selected.insert(pid);
                parent = parent_task.parent_task_id;
            }
        }
    }
    let mut days = Vec::new();
    let mut week_estimate_remaining = 0i64;
    let mut week_tracked = 0i64;
    let mut unscheduled = Vec::new();

    for task in tasks {
        if task.scheduled_date.is_some() && !selected.contains(&task.id) {
            continue;
        }
        let context_only = selected.contains(&task.id) && task.scheduled_date.as_deref().is_none();
        let week_task = to_week_task(conn, task, &today, context_only)?;
        match &week_task.scheduled_date {
            Some(date)
                if date.as_str() >= start.to_string().as_str()
                    && date.as_str() <= end.to_string().as_str() => {}
            Some(_) => continue,
            None => {
                if week_task.context_only {
                    continue;
                }
                let lookahead = end + Duration::days(7);
                let in_range = week_task
                    .due_at
                    .as_ref()
                    .map(|d| {
                        let d = &d[..10.min(d.len())];
                        d <= lookahead.to_string().as_str()
                    })
                    .unwrap_or(false);
                if in_range {
                    unscheduled.push(week_task);
                }
                continue;
            }
        }
        if !week_task.context_only {
            if let Some(minutes) = week_task.remaining_minutes {
                week_estimate_remaining += minutes;
            }
        }
        if !week_task.context_only {
            week_tracked += week_task.tracked_seconds_direct;
        }
        days.push(week_task);
    }

    let mut per_day: Vec<WeekDay> = Vec::new();
    for offset in 0..7 {
        let date = start + Duration::days(offset);
        let date_str = date.to_string();
        let weekday = date.format("%u").to_string().parse::<u32>().unwrap_or(1);
        let day_tasks: Vec<WeekTask> = days
            .iter()
            .filter(|t| t.scheduled_date.as_deref() == Some(date_str.as_str()))
            .map(|t| WeekTask {
                id: t.id,
                class_id: t.class_id,
                course_code: t.course_code.clone(),
                title: t.title.clone(),
                status: t.status.clone(),
                due_at: t.due_at.clone(),
                scheduled_date: t.scheduled_date.clone(),
                estimated_minutes: t.estimated_minutes,
                tracked_seconds_direct: t.tracked_seconds_direct,
                tracked_seconds_total: t.tracked_seconds_total,
                remaining_minutes: t.remaining_minutes,
                overdue: t.overdue,
                context_only: t.context_only,
            })
            .collect();
        let estimated_minutes_remaining =
            day_tasks.iter().filter_map(|t| t.remaining_minutes).sum();
        let tracked_seconds = day_tasks.iter().map(|t| t.tracked_seconds_direct).sum();
        let capacity_minutes = capacity_for(conn, &date_str, weekday)?;
        let load_percent = capacity_minutes
            .filter(|c| *c > 0)
            .map(|c| (estimated_minutes_remaining * 100) / c);
        per_day.push(WeekDay {
            date: date_str,
            tasks: day_tasks,
            estimated_minutes_remaining,
            tracked_seconds,
            capacity_minutes,
            load_percent,
        });
    }

    Ok(WeekSummary {
        start_date: start.to_string(),
        end_date: end.to_string(),
        days: per_day,
        unscheduled,
        estimated_minutes_remaining: week_estimate_remaining,
        tracked_seconds: week_tracked,
    })
}

#[tauri::command]
pub fn get_week(db: State<Db>, start_date: String) -> Result<WeekSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    week_for(&conn, &start_date).map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct CapacitySettings {
    pub weekday_minutes: [Option<i64>; 7],
}

#[tauri::command]
pub fn get_study_capacity(db: State<Db>) -> Result<CapacitySettings, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut weekday_minutes = [None; 7];
    for (i, slot) in weekday_minutes.iter_mut().enumerate() {
        let key = format!("study_capacity_{}", i + 1);
        *slot = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM app_settings WHERE key=?1",
                [key],
                |r| r.get(0),
            )
            .ok();
    }
    Ok(CapacitySettings { weekday_minutes })
}

#[tauri::command]
pub fn set_study_capacity(db: State<Db>, weekday_minutes: [Option<i64>; 7]) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    for (i, minutes) in weekday_minutes.iter().enumerate() {
        let key = format!("study_capacity_{}", i + 1);
        match minutes {
            Some(m) => conn
                .execute(
                    "INSERT INTO app_settings(key,value) VALUES(?1,?2)
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                    rusqlite::params![key, m.to_string()],
                )
                .map_err(|e| e.to_string())?,
            None => conn
                .execute("DELETE FROM app_settings WHERE key=?1", [key])
                .map_err(|e| e.to_string())?,
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = crate::db::test_connection();
        c.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall')")
            .unwrap();
        c
    }

    fn add(
        c: &Connection,
        id: i64,
        due: Option<&str>,
        scheduled: Option<&str>,
        estimate: Option<i64>,
    ) {
        c.execute(
            "INSERT INTO tasks(id,class_id,title,due_at,scheduled_date,estimated_minutes) VALUES(?1,1,'Task',?2,?3,?4)",
            rusqlite::params![id, due, scheduled, estimate],
        )
        .unwrap();
    }

    #[test]
    fn scheduled_task_lands_on_correct_day() {
        let c = db();
        add(&c, 1, Some("2026-09-17"), Some("2026-09-15"), Some(60));
        let week = week_for(&c, "2026-09-14").unwrap();
        assert_eq!(week.days[1].date, "2026-09-15");
        assert_eq!(week.days[1].tasks.len(), 1);
        assert_eq!(week.days[1].tasks[0].due_at.as_deref(), Some("2026-09-17"));
    }

    #[test]
    fn unscheduled_due_within_lookahead_is_surfaced() {
        let c = db();
        add(&c, 1, Some("2026-09-18"), None, Some(30));
        let week = week_for(&c, "2026-09-14").unwrap();
        assert_eq!(week.unscheduled.len(), 1);
        assert!(week.days.iter().all(|d| d.tasks.is_empty()));
    }

    #[test]
    fn unscheduled_far_future_not_surfaced() {
        let c = db();
        add(&c, 1, Some("2026-12-01"), None, Some(30));
        let week = week_for(&c, "2026-09-14").unwrap();
        assert!(week.unscheduled.is_empty());
    }

    #[test]
    fn remaining_minutes_clamped_at_zero() {
        let c = db();
        add(&c, 1, None, Some("2026-09-15"), Some(30));
        c.execute_batch(
            "INSERT INTO time_sessions(task_id,start_ts,end_ts,final_duration_seconds)
             VALUES(1,datetime('2026-09-15 10:00'),datetime('2026-09-15 11:00'),3600)",
        )
        .unwrap();
        let week = week_for(&c, "2026-09-14").unwrap();
        assert_eq!(week.days[1].tasks[0].remaining_minutes, Some(0));
    }

    #[test]
    fn capacity_override_wins_over_weekday_default() {
        let c = db();
        c.execute(
            "INSERT INTO app_settings(key,value) VALUES('study_capacity_2','120')",
            [],
        )
        .unwrap();
        c.execute(
            "INSERT INTO study_capacity_overrides(date,available_minutes) VALUES('2026-09-15',45)",
            [],
        )
        .unwrap();
        let week = week_for(&c, "2026-09-14").unwrap();
        assert_eq!(week.days[1].capacity_minutes, Some(45));
    }
}
