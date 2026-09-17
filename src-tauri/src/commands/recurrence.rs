use crate::commands::tasks::{row_to_task, Task, ELIGIBLE_TASK_CLAUSE, TASK_SELECT};
use crate::db::Db;
use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringTemplate {
    pub id: i64,
    pub class_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub priority: i64,
    pub estimated_minutes: Option<i64>,
    pub recurrence_type: String,
    pub interval: i64,
    pub weekdays: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub active: bool,
}
#[derive(Debug, Deserialize)]
pub struct NewRecurringTemplate {
    pub class_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i64>,
    pub estimated_minutes: Option<i64>,
    pub recurrence_type: String,
    pub interval: Option<i64>,
    pub weekdays: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct UpdateRecurringTemplate {
    pub id: i64,
    pub class_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub priority: Option<i64>,
    pub estimated_minutes: Option<i64>,
    pub recurrence_type: String,
    pub interval: Option<i64>,
    pub weekdays: Option<String>,
    pub start_date: String,
    pub end_date: Option<String>,
}
fn template_row(row: &rusqlite::Row) -> rusqlite::Result<RecurringTemplate> {
    Ok(RecurringTemplate {
        id: row.get(0)?,
        class_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        priority: row.get(4)?,
        estimated_minutes: row.get(5)?,
        recurrence_type: row.get(6)?,
        interval: row.get(7)?,
        weekdays: row.get(8)?,
        start_date: row.get(9)?,
        end_date: row.get(10)?,
        active: row.get::<_, i64>(11)? != 0,
    })
}
fn valid_date(v: &str) -> bool {
    NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok()
}
fn validate(i: &NewRecurringTemplate) -> Result<(), String> {
    if i.title.trim().is_empty() {
        return Err("Enter a recurring task title.".into());
    }
    if !["daily", "weekly", "weekdays"].contains(&i.recurrence_type.as_str()) {
        return Err("Choose a valid recurrence.".into());
    }
    if i.interval.unwrap_or(1) < 1
        || i.priority.unwrap_or(2) < 1
        || i.priority.unwrap_or(2) > 4
        || i.estimated_minutes.is_some_and(|n| n < 0)
    {
        return Err("Check the recurrence interval, priority, and estimate.".into());
    }
    if !valid_date(&i.start_date)
        || i.end_date.as_deref().is_some_and(|d| !valid_date(d))
        || i.end_date.as_ref().is_some_and(|d| d < &i.start_date)
    {
        return Err("Choose a valid date range.".into());
    }
    Ok(())
}
fn should_generate(t: &RecurringTemplate, date: NaiveDate) -> bool {
    let start = NaiveDate::parse_from_str(&t.start_date, "%Y-%m-%d").unwrap();
    if date < start
        || t.end_date
            .as_ref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .is_some_and(|e| date > e)
    {
        return false;
    }
    let days = (date - start).num_days();
    match t.recurrence_type.as_str() {
        "daily" => days % t.interval == 0,
        "weekly" => days % 7 == 0 && (days / 7) % t.interval == 0,
        "weekdays" => {
            let selected = t.weekdays.as_deref().unwrap_or("1,2,3,4,5");
            let weekday = date.weekday().number_from_monday() as i64;
            let selected = selected
                .split(',')
                .filter_map(|v| v.trim().parse::<i64>().ok())
                .any(|v| v == weekday);
            selected && (days / 7) % t.interval == 0
        }
        _ => false,
    }
}
pub(crate) fn ensure_generated(conn: &Connection, horizon_days: i64) -> rusqlite::Result<usize> {
    let today = chrono::Local::now().date_naive();
    let horizon = today + Duration::days(horizon_days);
    let ts: Vec<RecurringTemplate> = conn.prepare("SELECT id,class_id,title,description,priority,estimated_minutes,recurrence_type,interval,weekdays,start_date,end_date,active FROM recurring_task_templates WHERE active=1")?.query_map([], template_row)?.collect::<Result<_,_>>()?;
    let mut count = 0;
    for t in ts {
        let mut d = today.max(NaiveDate::parse_from_str(&t.start_date, "%Y-%m-%d").unwrap());
        while d <= horizon {
            if should_generate(&t, d) {
                count += conn.execute("INSERT OR IGNORE INTO tasks(class_id,title,description,priority,scheduled_date,estimated_minutes,recurring_template_id,occurrence_date) VALUES(?1,?2,?3,?4,?5,?6,?7,?5)", params![t.class_id,t.title,t.description,t.priority,d.to_string(),t.estimated_minutes,t.id])?;
            }
            d += Duration::days(1);
        }
    }
    Ok(count)
}
const TEMPLATE_SQL: &str = "SELECT id,class_id,title,description,priority,estimated_minutes,recurrence_type,interval,weekdays,start_date,end_date,active FROM recurring_task_templates";
#[tauri::command]
pub fn list_recurring_templates(db: State<Db>) -> Result<Vec<RecurringTemplate>, String> {
    let c = db.0.lock().map_err(|_| "Couldn't load recurring tasks")?;
    let result = c
        .prepare(&format!(
            "{TEMPLATE_SQL} ORDER BY active DESC,start_date,id"
        ))
        .map_err(|e| e.to_string())?
        .query_map([], template_row)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string());
    result
}
#[tauri::command]
pub fn create_recurring_template(
    db: State<Db>,
    input: NewRecurringTemplate,
) -> Result<RecurringTemplate, String> {
    validate(&input)?;
    let c = db.0.lock().map_err(|_| "Couldn't save recurring task")?;
    c.execute("INSERT INTO recurring_task_templates(class_id,title,description,priority,estimated_minutes,recurrence_type,interval,weekdays,start_date,end_date) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![input.class_id,input.title.trim(),input.description,input.priority.unwrap_or(2),input.estimated_minutes,input.recurrence_type,input.interval.unwrap_or(1),input.weekdays,input.start_date,input.end_date]).map_err(|_| "Couldn't save recurring task")?;
    let id = c.last_insert_rowid();
    ensure_generated(&c, 45).map_err(|_| "Couldn't generate recurring tasks")?;
    c.query_row(&format!("{TEMPLATE_SQL} WHERE id=?1"), [id], template_row)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn set_recurring_template_active(db: State<Db>, id: i64, active: bool) -> Result<(), String> {
    let c = db.0.lock().map_err(|_| "Couldn't update recurring task")?;
    let tx = c
        .unchecked_transaction()
        .map_err(|_| "Couldn't update recurring task")?;
    tx.execute(
        "UPDATE recurring_task_templates SET active=?1,updated_at=datetime('now') WHERE id=?2",
        params![active, id],
    )
    .map_err(|_| "Couldn't update recurring task")?;
    if !active {
        tx.execute("DELETE FROM tasks WHERE recurring_template_id=?1 AND status!='completed' AND occurrence_date >= date('now','localtime') AND NOT EXISTS (SELECT 1 FROM time_sessions WHERE task_id=tasks.id)", [id]).map_err(|_| "Couldn't clean recurring tasks")?;
    }
    tx.commit().map_err(|_| "Couldn't update recurring task")?;
    Ok(())
}

#[tauri::command]
pub fn update_recurring_template(
    db: State<Db>,
    input: UpdateRecurringTemplate,
) -> Result<RecurringTemplate, String> {
    let new = NewRecurringTemplate {
        class_id: input.class_id,
        title: input.title.clone(),
        description: input.description.clone(),
        priority: input.priority,
        estimated_minutes: input.estimated_minutes,
        recurrence_type: input.recurrence_type.clone(),
        interval: input.interval,
        weekdays: input.weekdays.clone(),
        start_date: input.start_date.clone(),
        end_date: input.end_date.clone(),
    };
    validate(&new)?;
    let c = db.0.lock().map_err(|_| "Couldn't update recurring task")?;
    let tx = c
        .unchecked_transaction()
        .map_err(|_| "Couldn't update recurring task")?;
    tx.execute("UPDATE recurring_task_templates SET class_id=?1,title=?2,description=?3,priority=?4,estimated_minutes=?5,recurrence_type=?6,interval=?7,weekdays=?8,start_date=?9,end_date=?10,updated_at=datetime('now') WHERE id=?11", params![new.class_id,new.title.trim(),new.description,new.priority.unwrap_or(2),new.estimated_minutes,new.recurrence_type,new.interval.unwrap_or(1),new.weekdays,new.start_date,new.end_date,input.id]).map_err(|_| "Couldn't update recurring task")?;
    tx.execute("DELETE FROM tasks WHERE recurring_template_id=?1 AND status!='completed' AND occurrence_date >= date('now','localtime') AND NOT EXISTS (SELECT 1 FROM time_sessions WHERE task_id=tasks.id)", [input.id]).map_err(|_| "Couldn't update recurring occurrences")?;
    tx.commit().map_err(|_| "Couldn't update recurring task")?;
    ensure_generated(&c, 45).map_err(|_| "Couldn't generate recurring tasks")?;
    c.query_row(
        &format!("{TEMPLATE_SQL} WHERE id=?1"),
        [input.id],
        template_row,
    )
    .map_err(|e| e.to_string())
}

#[derive(Debug, Serialize)]
pub struct CalendarDay {
    pub date: String,
    pub planned: Vec<Task>,
    pub due: Vec<Task>,
    pub study_blocks: Vec<crate::commands::planning::StudyBlock>,
}
#[tauri::command]
pub fn get_calendar(db: State<Db>, month: String) -> Result<Vec<CalendarDay>, String> {
    let first = NaiveDate::parse_from_str(&(month + "-01"), "%Y-%m-%d")
        .map_err(|_| "Choose a valid month")?;
    let next = first
        .checked_add_months(chrono::Months::new(1))
        .ok_or("Invalid month")?;
    let c = db.0.lock().map_err(|_| "Couldn't load calendar")?;
    ensure_generated(&c, 45).map_err(|_| "Couldn't prepare recurring tasks")?;
    let tasks:Vec<Task>=c.prepare(&format!("{TASK_SELECT} WHERE {ELIGIBLE_TASK_CLAUSE} AND t.status != 'completed' AND ((t.scheduled_date >= ?1 AND t.scheduled_date < ?2) OR (substr(t.due_at,1,10) >= ?1 AND substr(t.due_at,1,10) < ?2)) ORDER BY t.id")).map_err(|_| "Couldn't load calendar tasks")?.query_map(params![first.to_string(),next.to_string()],row_to_task).map_err(|_| "Couldn't load calendar tasks")?.collect::<Result<_,_>>().map_err(|_| "Couldn't load calendar tasks")?;
    let study_blocks = crate::commands::planning::blocks_for_range(
        &c,
        &first.to_string(),
        &(next - Duration::days(1)).to_string(),
    )
    .map_err(|_| "Couldn't load calendar study blocks")?;
    let mut out = Vec::new();
    let mut d = first;
    while d < next {
        let key = d.to_string();
        let planned = tasks
            .iter()
            .filter(|t| t.scheduled_date.as_deref() == Some(key.as_str()))
            .cloned()
            .collect();
        let due = tasks
            .iter()
            .filter(|t| t.due_at.as_deref().is_some_and(|v| v.starts_with(&key)))
            .cloned()
            .collect();
        out.push(CalendarDay {
            date: key.clone(),
            planned,
            due,
            study_blocks: study_blocks
                .iter()
                .filter(|block| block.planned_date == key)
                .cloned()
                .collect(),
        });
        d += Duration::days(1);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let c = crate::db::test_connection();
        c.execute(
            "INSERT INTO classes(id,course_code,semester) VALUES(1,'TEST','Fall')",
            [],
        )
        .unwrap();
        c
    }
    #[test]
    fn daily_occurrences_are_unique() {
        let c = db();
        c.execute("INSERT INTO recurring_task_templates(class_id,title,recurrence_type,start_date) VALUES(1,'Review','daily',?1)", [&chrono::Local::now().date_naive().to_string()]).unwrap();
        ensure_generated(&c, 3).unwrap();
        ensure_generated(&c, 3).unwrap();
        assert_eq!(
            c.query_row("SELECT count(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            4
        );
    }
}
