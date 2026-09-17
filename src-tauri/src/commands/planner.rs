use super::tasks::{row_to_task, Task, ELIGIBLE_TASK_CLAUSE, TASK_SELECT};
use crate::db::Db;
use chrono::{Duration, NaiveDate};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    pub start_date: String,
    pub days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProposedBlock {
    pub task_id: i64,
    pub task_title: String,
    pub course_code: String,
    pub planned_date: String,
    pub planned_minutes: i64,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct PlanDay {
    pub date: String,
    pub capacity_minutes: Option<i64>,
    pub existing_block_minutes: i64,
    pub proposed_minutes: i64,
    pub blocks: Vec<ProposedBlock>,
}

#[derive(Debug, Serialize)]
pub struct PlanProposal {
    pub start_date: String,
    pub end_date: String,
    pub days: Vec<PlanDay>,
    pub unschedulable_overdue: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApplyPlanInput {
    pub blocks: Vec<ProposedBlockInput>,
}

#[derive(Debug, Deserialize)]
pub struct ProposedBlockInput {
    pub task_id: i64,
    pub planned_date: String,
    pub planned_minutes: i64,
}

fn capacity_for(conn: &Connection, date: &str) -> rusqlite::Result<Option<i64>> {
    if let Ok(minutes) = conn.query_row(
        "SELECT available_minutes FROM study_capacity_overrides WHERE date=?1",
        [date],
        |row| row.get::<_, i64>(0),
    ) {
        return Ok(Some(minutes));
    }
    let weekday = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| rusqlite::Error::InvalidQuery)?
        .format("%u")
        .to_string();
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

fn task_due(task: &Task) -> Option<NaiveDate> {
    task.due_at
        .as_deref()
        .and_then(|date| NaiveDate::parse_from_str(&date[..10.min(date.len())], "%Y-%m-%d").ok())
}

fn existing_minutes(conn: &Connection, date: &str) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(planned_minutes),0) FROM study_blocks WHERE planned_date=?1 AND completed=0",
        [date],
        |row| row.get(0),
    )
}

fn course_codes(conn: &Connection) -> rusqlite::Result<HashMap<i64, String>> {
    conn.prepare("SELECT id,course_code FROM classes")?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect()
}

pub(crate) fn plan_proposal_for(
    conn: &Connection,
    start_date: &str,
    days: i64,
) -> Result<PlanProposal, String> {
    let start = NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
        .map_err(|_| "Choose a valid planning start date.".to_string())?;
    let days = days.clamp(1, 7);
    let end = start + Duration::days(days - 1);
    let class_codes = course_codes(conn).map_err(|e| e.to_string())?;
    let mut tasks: Vec<Task> = conn
        .prepare(&format!(
            "{TASK_SELECT} WHERE {ELIGIBLE_TASK_CLAUSE} AND t.status!='completed' ORDER BY t.id"
        ))
        .map_err(|e| e.to_string())?
        .query_map([], row_to_task)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    tasks.retain(|task| {
        task.remaining_minutes.unwrap_or(0) >= 25 && task.blocked_by_open_count == 0
    });
    tasks.sort_by_key(|task| {
        let due = task_due(task);
        (
            due.is_none(),
            due.unwrap_or(NaiveDate::MAX),
            -task.priority.unwrap_or(0),
            -task.unplanned_minutes,
            -task.remaining_minutes.unwrap_or(0),
            task.id,
        )
    });
    let mut remaining: HashMap<i64, i64> = tasks
        .iter()
        .map(|task| (task.id, task.remaining_minutes.unwrap_or(0)))
        .collect();
    let mut plan_days = Vec::new();
    let mut overdue = Vec::new();
    let mut date = start;
    while date <= end {
        let date_string = date.to_string();
        let capacity = capacity_for(conn, &date_string).map_err(|e| e.to_string())?;
        let existing = existing_minutes(conn, &date_string).map_err(|e| e.to_string())?;
        let mut available = capacity.map(|value| (value - existing).max(0)).unwrap_or(0);
        let mut blocks = Vec::new();
        for task in &tasks {
            let Some(task_remaining) = remaining.get_mut(&task.id) else {
                continue;
            };
            if *task_remaining < 25 || available < 25 {
                continue;
            }
            if let Some(due) = task_due(task) {
                if due < start {
                    if !overdue.contains(&task.title) {
                        overdue.push(task.title.clone());
                    }
                    continue;
                }
                if date > due {
                    continue;
                }
            }
            let chunk = (*task_remaining).min(90).min(available);
            let chunk = if chunk < 25 { continue } else { chunk };
            let reason = if task_due(task).is_some_and(|due| due <= date + Duration::days(2)) {
                "Deadline soon"
            } else if task.unplanned_minutes > 0 {
                "Planning gap"
            } else {
                "Remaining work"
            };
            blocks.push(ProposedBlock {
                task_id: task.id,
                task_title: task.title.clone(),
                course_code: class_codes.get(&task.class_id).cloned().unwrap_or_default(),
                planned_date: date_string.clone(),
                planned_minutes: chunk,
                reason: reason.into(),
            });
            *task_remaining -= chunk;
            available -= chunk;
            if available < 25 {
                break;
            }
        }
        plan_days.push(PlanDay {
            date: date_string,
            capacity_minutes: capacity,
            existing_block_minutes: existing,
            proposed_minutes: blocks.iter().map(|block| block.planned_minutes).sum(),
            blocks,
        });
        date += Duration::days(1);
    }
    Ok(PlanProposal {
        start_date: start.to_string(),
        end_date: end.to_string(),
        days: plan_days,
        unschedulable_overdue: overdue,
    })
}

#[tauri::command]
pub fn get_plan_proposal(db: State<Db>, input: PlanRequest) -> Result<PlanProposal, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_proposal_for(&conn, &input.start_date, input.days)
}

#[tauri::command]
pub fn apply_plan(db: State<Db>, input: ApplyPlanInput) -> Result<usize, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for block in &input.blocks {
        if block.planned_minutes < 25 {
            return Err("Plans cannot create blocks shorter than 25 minutes.".into());
        }
        NaiveDate::parse_from_str(&block.planned_date, "%Y-%m-%d")
            .map_err(|_| "Plan contains an invalid date.".to_string())?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1 AND status!='completed' AND is_class_timer=0)",
                [block.task_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err("Plan contains a task that is no longer available.".into());
        }
        tx.execute(
            "INSERT INTO study_blocks(task_id,planned_date,planned_minutes) VALUES(?1,?2,?3)",
            params![block.task_id, block.planned_date, block.planned_minutes],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(input.blocks.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let c = crate::db::test_connection();
        c.execute_batch(
            "INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall');
             INSERT INTO app_settings(key,value) VALUES
                ('study_capacity_1','120'),('study_capacity_2','120'),
                ('study_capacity_3','120'),('study_capacity_4','120'),
                ('study_capacity_5','120'),('study_capacity_6','120'),
                ('study_capacity_7','120');",
        )
        .unwrap();
        c
    }

    fn add(c: &Connection, id: i64, due: Option<&str>, estimate: i64) {
        c.execute(
            "INSERT INTO tasks(id,class_id,title,due_at,estimated_minutes) VALUES(?1,1,'Task',?2,?3)",
            params![id, due, estimate],
        )
        .unwrap();
    }

    #[test]
    fn blocked_task_is_excluded_from_the_proposal() {
        let c = db();
        add(&c, 1, Some("2026-09-20"), 60);
        add(&c, 2, Some("2026-09-20"), 60);
        c.execute(
            "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES(2,1)",
            [],
        )
        .unwrap();
        let plan = plan_proposal_for(&c, "2026-09-17", 3).unwrap();
        let scheduled_ids: Vec<i64> = plan
            .days
            .iter()
            .flat_map(|d| d.blocks.iter())
            .map(|b| b.task_id)
            .collect();
        assert!(scheduled_ids.contains(&1));
        assert!(!scheduled_ids.contains(&2));
    }

    #[test]
    fn task_becomes_schedulable_once_its_blocker_completes() {
        let c = db();
        add(&c, 1, Some("2026-09-20"), 60);
        add(&c, 2, Some("2026-09-20"), 60);
        c.execute(
            "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES(2,1)",
            [],
        )
        .unwrap();
        c.execute("UPDATE tasks SET status='completed' WHERE id=1", [])
            .unwrap();
        let plan = plan_proposal_for(&c, "2026-09-17", 3).unwrap();
        let scheduled_ids: Vec<i64> = plan
            .days
            .iter()
            .flat_map(|d| d.blocks.iter())
            .map(|b| b.task_id)
            .collect();
        assert!(scheduled_ids.contains(&2));
    }
}
