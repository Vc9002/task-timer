use super::tasks::{row_to_task, Task, TASK_SELECT};
use crate::db::Db;
use rusqlite::{params, Connection};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct TodayTask {
    #[serde(flatten)]
    pub task: Task,
    pub overdue: bool,
    pub context_only: bool,
}
#[derive(Debug, Serialize)]
pub struct TodayClassGroup {
    pub class_id: i64,
    pub course_code: String,
    pub tasks: Vec<TodayTask>,
}
#[derive(Debug, Serialize)]
pub struct TodaySummary {
    pub groups: Vec<TodayClassGroup>,
    pub task_count: i64,
    pub estimated_minutes_total: i64,
    pub tracked_seconds_total: i64,
}

pub(crate) fn today_for(
    conn: &Connection,
    date: &str,
    include_overdue: bool,
) -> rusqlite::Result<TodaySummary> {
    let eligible = "t.class_id IN (SELECT id FROM classes WHERE active=1) AND
        (t.source='local' OR (t.external_state='active' AND EXISTS
        (SELECT 1 FROM todoist_projects p WHERE p.todoist_id=t.todoist_project_id AND p.class_id=t.class_id)))";
    let tasks: Vec<Task> = conn
        .prepare(&format!("{TASK_SELECT} WHERE {eligible} ORDER BY t.id"))?
        .query_map([], row_to_task)?
        .collect::<Result<_, _>>()?;
    let by_id: HashMap<i64, &Task> = tasks.iter().map(|t| (t.id, t)).collect();
    let mut children: HashMap<i64, Vec<i64>> = HashMap::new();
    for task in &tasks {
        if let Some(parent) = task.parent_task_id {
            children.entry(parent).or_default().push(task.id);
        }
    }
    // Date-only due dates are floating local dates; timestamp deadlines use local time.
    let seeds: Vec<i64> = conn.prepare(&format!("SELECT t.id FROM tasks t WHERE {eligible} AND t.status!='completed' AND (
        CASE WHEN length(t.due_at)=10 THEN date(t.due_at) ELSE date(t.due_at,'localtime') END = ?1
        OR (?2 AND CASE WHEN length(t.due_at)=10 THEN date(t.due_at) ELSE date(t.due_at,'localtime') END < ?1)
        OR t.scheduled_date=?1)"))?.query_map(params![date,include_overdue], |r|r.get(0))?.collect::<Result<_,_>>()?;
    let mut actionable = HashSet::new();
    let mut pending = seeds;
    while let Some(id) = pending.pop() {
        if !actionable.insert(id) {
            continue;
        }
        if let Some(ids) = children.get(&id) {
            pending.extend(ids.iter().filter(|id| by_id[id].status != "completed"));
        }
    }
    let mut included = actionable.clone();
    for id in &actionable {
        let mut parent = by_id[id].parent_task_id;
        while let Some(pid) = parent {
            let Some(task) = by_id.get(&pid) else {
                break;
            };
            if !included.insert(pid) {
                break;
            }
            parent = task.parent_task_id;
        }
    }
    let mut groups: BTreeMap<i64, TodayClassGroup> = BTreeMap::new();
    let mut estimate = 0;
    for task in tasks {
        if !included.contains(&task.id) {
            continue;
        }
        let context_only = !actionable.contains(&task.id);
        if !context_only {
            estimate += task.estimated_minutes.unwrap_or(0);
        }
        let overdue = match &task.due_at {
            Some(due) => conn.query_row("SELECT CASE WHEN length(?1)=10 THEN date(?1) ELSE date(?1,'localtime') END < ?2",params![due,date],|r|r.get::<_,Option<bool>>(0))?.unwrap_or(false),
            None => false,
        };
        let code: String = conn.query_row(
            "SELECT course_code FROM classes WHERE id=?1",
            [task.class_id],
            |r| r.get(0),
        )?;
        groups
            .entry(task.class_id)
            .or_insert_with(|| TodayClassGroup {
                class_id: task.class_id,
                course_code: code,
                tasks: vec![],
            })
            .tasks
            .push(TodayTask {
                task,
                overdue,
                context_only,
            });
    }
    // This header is actual time tracked today, including completed/archived work.
    // Every session counts once, regardless of hierarchy or current task visibility.
    let tracked_seconds_total = conn.query_row("SELECT COALESCE(SUM(final_duration_seconds),0) FROM time_sessions WHERE end_ts IS NOT NULL AND date(start_ts,'localtime')=?1",[date],|r|r.get(0))?;
    let mut groups: Vec<_> = groups.into_values().collect();
    groups.sort_by(|a, b| a.course_code.cmp(&b.course_code));
    Ok(TodaySummary {
        groups,
        task_count: actionable.len() as i64,
        estimated_minutes_total: estimate,
        tracked_seconds_total,
    })
}

#[tauri::command]
pub fn get_today(db: State<Db>, include_overdue: bool) -> Result<TodaySummary, String> {
    let conn = db.0.lock().map_err(|_| "Couldn't load Today")?;
    today_for(
        &conn,
        &chrono::Local::now().format("%Y-%m-%d").to_string(),
        include_overdue,
    )
    .map_err(|e| {
        eprintln!("Today: {e}");
        "Couldn't load Today. Try again.".into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let c = crate::db::test_connection();
        c.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall');")
            .unwrap();
        c
    }
    fn add(
        c: &Connection,
        id: i64,
        parent: Option<i64>,
        due: Option<&str>,
        schedule: Option<&str>,
    ) {
        c.execute("INSERT INTO tasks(id,class_id,title,parent_task_id,due_at,scheduled_date) VALUES(?1,1,'Task',?2,?3,?4)",params![id,parent,due,schedule]).unwrap();
    }
    fn ids(c: &Connection) -> Vec<i64> {
        today_for(c, "2026-09-15", true)
            .unwrap()
            .groups
            .into_iter()
            .flat_map(|g| g.tasks.into_iter().map(|t| t.task.id))
            .collect()
    }
    #[test]
    fn parent_due_includes_undated_child() {
        let c = db();
        add(&c, 1, None, Some("2026-09-15"), None);
        add(&c, 2, Some(1), None, None);
        assert_eq!(ids(&c), vec![1, 2]);
    }
    #[test]
    fn child_due_includes_context_without_siblings() {
        let c = db();
        add(&c, 1, None, Some("2026-10-01"), None);
        add(&c, 2, Some(1), Some("2026-09-15"), None);
        add(&c, 3, Some(1), None, None);
        assert_eq!(ids(&c), vec![1, 2]);
        assert_eq!(today_for(&c, "2026-09-15", true).unwrap().task_count, 1);
    }
    #[test]
    fn deep_tree() {
        let c = db();
        add(&c, 1, None, None, None);
        add(&c, 2, Some(1), None, None);
        add(&c, 3, Some(2), None, Some("2026-09-15"));
        add(&c, 4, Some(3), None, None);
        assert_eq!(ids(&c), vec![1, 2, 3, 4]);
    }
    #[test]
    fn overdue() {
        let c = db();
        add(&c, 1, None, Some("2026-09-01"), None);
        assert_eq!(ids(&c), vec![1]);
        assert!(today_for(&c, "2026-09-15", false)
            .unwrap()
            .groups
            .is_empty());
    }
    #[test]
    fn scheduled_today() {
        let c = db();
        add(&c, 1, None, Some("2026-10-01"), Some("2026-09-15"));
        assert_eq!(ids(&c), vec![1]);
    }
    #[test]
    fn future_not_scheduled() {
        let c = db();
        add(&c, 1, None, Some("2026-10-01"), None);
        assert!(ids(&c).is_empty());
    }
    #[test]
    fn archived_hidden() {
        let c = db();
        add(&c, 1, None, None, Some("2026-09-15"));
        c.execute("UPDATE classes SET active=0", []).unwrap();
        assert!(ids(&c).is_empty());
    }
    #[test]
    fn completed_hidden() {
        let c = db();
        add(&c, 1, None, None, Some("2026-09-15"));
        c.execute("UPDATE tasks SET status='completed'", [])
            .unwrap();
        assert!(ids(&c).is_empty());
    }
    #[test]
    fn completed_ancestor_keeps_incomplete_child_visible() {
        let c = db();
        add(&c, 1, None, None, None);
        add(&c, 2, Some(1), None, Some("2026-09-15"));
        c.execute("UPDATE tasks SET status='completed' WHERE id=1", [])
            .unwrap();
        assert_eq!(ids(&c), vec![1, 2]);
    }
    #[test]
    fn today_total_counts_each_session_once_including_completed_work() {
        let c = db();
        add(&c, 1, None, Some("2026-09-15"), None);
        add(&c, 2, Some(1), None, None);
        c.execute_batch("INSERT INTO time_sessions(task_id,start_ts,end_ts,final_duration_seconds) VALUES(1,datetime('2026-09-15 12:00','utc'),datetime('2026-09-15 12:30','utc'),1800),(2,datetime('2026-09-15 13:00','utc'),datetime('2026-09-15 13:40','utc'),2400)").unwrap();
        let summary = today_for(&c, "2026-09-15", true).unwrap();
        assert_eq!(summary.tracked_seconds_total, 4200);
        assert_eq!(summary.groups[0].tasks[0].task.tracked_seconds, 4200);
        c.execute("UPDATE tasks SET status='completed'", [])
            .unwrap();
        assert_eq!(
            today_for(&c, "2026-09-15", true)
                .unwrap()
                .tracked_seconds_total,
            4200
        );
    }
}
