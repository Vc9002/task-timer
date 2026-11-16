use super::tasks::{tasks_by_ids, Task, ELIGIBLE_TASK_CLAUSE};
use crate::commands::planning::StudyBlock;
use crate::db::Db;
use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
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
    pub parent_path: Option<String>,
    pub scheduled_minutes_before_due: i64,
    pub unplanned_minutes: i64,
    pub schedule_coverage_percent: i64,
}

#[derive(Debug, Serialize)]
pub struct WeekDay {
    pub date: String,
    pub tasks: Vec<WeekTask>,
    pub estimated_minutes_remaining: i64,
    pub tracked_seconds: i64,
    pub capacity_minutes: Option<i64>,
    pub load_percent: Option<i64>,
    pub study_blocks: Vec<StudyBlock>,
    pub study_block_minutes: i64,
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

/// The hierarchy fields of an open, eligible task, from the cheap first scan.
struct Outline {
    parent: Option<i64>,
    title: String,
    scheduled_date: Option<String>,
    due_at: Option<String>,
}

fn to_week_task(
    conn: &Connection,
    task: Task,
    context_only: bool,
    outline: &std::collections::HashMap<i64, Outline>,
    class_codes: &std::collections::HashMap<i64, String>,
    overdue_map: &std::collections::HashMap<String, bool>,
) -> rusqlite::Result<WeekTask> {
    let course_code = class_codes.get(&task.class_id).cloned().unwrap_or_default();
    let overdue = task
        .due_at
        .as_deref()
        .and_then(|due| overdue_map.get(due))
        .copied()
        .unwrap_or(false);
    let not_completed = task.status != "completed";
    let mut path = Vec::new();
    let mut parent = task.parent_task_id;
    while let Some(pid) = parent {
        // Open ancestors are already in memory. A completed or otherwise
        // filtered-out ancestor still belongs in the path, so look it up.
        let (title, next): (String, Option<i64>) = match outline.get(&pid) {
            Some(node) => (node.title.clone(), node.parent),
            None => conn.query_row(
                "SELECT title,parent_task_id FROM tasks WHERE id=?1",
                [pid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?,
        };
        path.push(title);
        parent = next;
    }
    path.reverse();
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
        parent_path: (!path.is_empty()).then(|| path.join(" › ")),
        scheduled_minutes_before_due: task.scheduled_minutes_before_due,
        unplanned_minutes: task.unplanned_minutes,
        schedule_coverage_percent: task.schedule_coverage_percent,
    })
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

    let eligible = ELIGIBLE_TASK_CLAUSE;
    let (start_s, end_s) = (start.to_string(), end.to_string());
    let lookahead = (end + Duration::days(7)).to_string();
    let study_blocks = crate::commands::planning::blocks_for_range(conn, &start_s, &end_s)?;
    // Cheap scan of every open task first; full rows are loaded only for shown tasks.
    let outline: std::collections::HashMap<i64, Outline> = conn
        .prepare(&format!(
            "SELECT t.id, t.parent_task_id, t.title, t.scheduled_date, t.due_at FROM tasks t
             WHERE {eligible} AND t.status != 'completed'"
        ))?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                Outline {
                    parent: r.get(1)?,
                    title: r.get(2)?,
                    scheduled_date: r.get(3)?,
                    due_at: r.get(4)?,
                },
            ))
        })?
        .collect::<Result<_, _>>()?;
    let in_week = |date: &str| date >= start_s.as_str() && date <= end_s.as_str();
    // Tasks scheduled this week, plus their ancestors (shown only as context).
    let mut selected = std::collections::HashSet::new();
    for (id, node) in &outline {
        if node.scheduled_date.as_deref().is_some_and(in_week) {
            selected.insert(*id);
            let mut parent = node.parent;
            while let Some(pid) = parent {
                let Some(parent_node) = outline.get(&pid) else {
                    break;
                };
                selected.insert(pid);
                parent = parent_node.parent;
            }
        }
    }
    // Shown: scheduled this week, or unscheduled and due within the lookahead.
    let mut shown: Vec<i64> = outline
        .iter()
        .filter(|(id, node)| match node.scheduled_date.as_deref() {
            Some(date) => in_week(date),
            None => {
                !selected.contains(*id)
                    && node
                        .due_at
                        .as_deref()
                        .is_some_and(|d| &d[..10.min(d.len())] <= lookahead.as_str())
            }
        })
        .map(|(id, _)| *id)
        .collect();
    shown.sort_unstable();
    let tasks: Vec<Task> = tasks_by_ids(conn, &shown)?;
    // Prefetch class codes and overdue status once instead of per task (avoids N+1 round-trips).
    let class_codes: std::collections::HashMap<i64, String> = conn
        .prepare("SELECT id, course_code FROM classes")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let overdue_map = crate::commands::today::overdue_map_for(
        conn,
        tasks.iter().filter_map(|t| t.due_at.as_deref()),
        &today,
    )?;

    let mut days = Vec::new();
    let mut week_estimate_remaining = 0i64;
    let mut week_tracked = 0i64;
    let mut unscheduled = Vec::new();

    for task in tasks {
        let week_task = to_week_task(conn, task, false, &outline, &class_codes, &overdue_map)?;
        if week_task.scheduled_date.is_none() {
            unscheduled.push(week_task);
            continue;
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
            .cloned()
            .collect();
        let estimated_minutes_remaining =
            day_tasks.iter().filter_map(|t| t.remaining_minutes).sum();
        let tracked_seconds = day_tasks.iter().map(|t| t.tracked_seconds_direct).sum();
        let capacity_minutes = capacity_for(conn, &date_str, weekday)?;
        let load_percent = capacity_minutes
            .filter(|c| *c > 0)
            .map(|c| (estimated_minutes_remaining * 100) / c);
        let day_blocks: Vec<StudyBlock> = study_blocks
            .iter()
            .filter(|block| block.planned_date == date_str)
            .cloned()
            .collect();
        let study_block_minutes = day_blocks.iter().map(|block| block.planned_minutes).sum();
        per_day.push(WeekDay {
            date: date_str,
            tasks: day_tasks,
            estimated_minutes_remaining,
            tracked_seconds,
            capacity_minutes,
            load_percent,
            study_blocks: day_blocks,
            study_block_minutes,
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

#[tauri::command(async)]
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

    #[test]
    fn study_blocks_are_returned_on_their_planned_day() {
        let c = db();
        add(&c, 1, Some("2026-09-20"), None, Some(90));
        c.execute("INSERT INTO study_blocks(task_id,planned_date,planned_start_time,planned_minutes) VALUES(1,'2026-09-15','18:00',60)", []).unwrap();
        let week = week_for(&c, "2026-09-14").unwrap();
        assert_eq!(week.days[1].study_blocks.len(), 1);
        assert_eq!(week.days[1].study_block_minutes, 60);
        assert_eq!(
            week.days[1].study_blocks[0].planned_start_time.as_deref(),
            Some("18:00")
        );
    }

    #[test]
    fn schedule_gap_uses_blocks_before_due_and_direct_tracking() {
        let c = db();
        add(&c, 1, Some("2026-09-20"), None, Some(120));
        c.execute("INSERT INTO study_blocks(task_id,planned_date,planned_minutes) VALUES(1,'2026-09-15',60)", []).unwrap();
        c.execute("INSERT INTO time_sessions(task_id,start_ts,end_ts,final_duration_seconds) VALUES(1,'2026-09-14','2026-09-14 00:30',1800)", []).unwrap();
        let week = week_for(&c, "2026-09-14").unwrap();
        assert_eq!(week.unscheduled[0].remaining_minutes, Some(90));
        assert_eq!(week.unscheduled[0].scheduled_minutes_before_due, 60);
        assert_eq!(week.unscheduled[0].unplanned_minutes, 30);
        assert_eq!(week.unscheduled[0].schedule_coverage_percent, 66);
    }

    #[test]
    fn parent_path_includes_open_and_completed_ancestors() {
        let c = db();
        c.execute_batch(
            "INSERT INTO tasks(id,class_id,title,status) VALUES(1,1,'Course project','completed');
             INSERT INTO tasks(id,class_id,title,parent_task_id) VALUES(2,1,'Draft',1);
             INSERT INTO tasks(id,class_id,title,parent_task_id,scheduled_date) VALUES(3,1,'Intro',2,'2026-09-15');",
        )
        .unwrap();
        let week = week_for(&c, "2026-09-14").unwrap();
        let task = &week.days[1].tasks[0];
        assert_eq!(task.id, 3);
        // The completed grandparent is not in the open-task scan but still belongs in the path.
        assert_eq!(task.parent_path.as_deref(), Some("Course project › Draft"));
        // The open parent is context for a scheduled task, so it is not listed as unscheduled.
        assert!(week.unscheduled.is_empty());
    }
}
