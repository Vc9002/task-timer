use super::client::{RemoteSync, TodoistClient, TodoistTask};
use crate::db::Db;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

// Serializes sync and credential changes, independently of the SQLite mutex.
static SYNC_GATE: Mutex<()> = Mutex::new(());
pub fn gate() -> Result<MutexGuard<'static, ()>, String> {
    SYNC_GATE
        .try_lock()
        .map_err(|_| "Todoist sync is already running. Try again when it finishes.".into())
}
fn database_error(error: impl std::fmt::Display) -> String {
    eprintln!("Todoist local apply failed: {error}");
    "TaskTimer couldn't save the Todoist sync. No partial changes were saved. Try again.".into()
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub projects_synced: usize,
    pub tasks_synced: usize,
}

pub fn sync_now(db: &Db) -> Result<SyncResult, String> {
    let _gate = gate()?;
    let token = super::get_token()?.ok_or("No Todoist token configured")?;
    let result = sync_with(db, |cursor| {
        let client = TodoistClient::new(token).map_err(|e| e.to_string())?;
        let first = client.fetch(cursor).map_err(|e| e.to_string())?;
        if first.full_sync {
            // Full snapshots may be cached by Todoist; catch up before applying.
            let next = client.fetch(&first.sync_token).map_err(|e| e.to_string())?;
            Ok(vec![first, next])
        } else {
            Ok(vec![first])
        }
    })?;
    flush_outbox_unlocked(db)?;
    Ok(result)
}

pub fn queue_completion(conn: &Connection, task_id: i64) -> Result<(), String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| "Couldn't start task completion".to_string())?;
    let (external_id, source): (Option<String>, String) = tx
        .query_row(
            "SELECT external_id,source FROM tasks WHERE id=?1",
            [task_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| "Task not found")?;
    if source != "todoist" || external_id.is_none() {
        return Err("This task is not a Todoist task.".into());
    }
    let external_id = external_id.unwrap();
    let already_queued: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM integration_outbox WHERE external_id=?1 AND status IN ('pending','sending','completed'))", [&external_id], |r| r.get(0)).map_err(|_| "Couldn't inspect Todoist completion")?;
    if already_queued {
        tx.commit()
            .map_err(|_| "Couldn't save task completion".to_string())?;
        return Ok(());
    }
    let uuid = Uuid::new_v4().to_string();
    let payload = serde_json::json!({"type":"item_complete","uuid":uuid,"args":{"id":external_id}});
    tx.execute("INSERT INTO integration_outbox(provider,entity_type,external_id,command_uuid,command_type,payload_json) VALUES('todoist','task',?1,?2,'item_complete',?3)", rusqlite::params![external_id, uuid, payload.to_string()]).map_err(|_| "Couldn't queue Todoist completion")?;
    tx.execute("UPDATE tasks SET status='completed',completed_at=datetime('now'),updated_at=datetime('now') WHERE id=?1", [task_id]).map_err(|_| "Couldn't save task completion")?;
    tx.commit()
        .map_err(|_| "Couldn't save task completion".to_string())?;
    Ok(())
}

pub fn flush_outbox(db: &Db) -> Result<(), String> {
    let _gate = gate()?;
    flush_outbox_unlocked(db)
}

pub fn recover_inflight_outbox(db: &Db) -> Result<(), String> {
    let conn = db.0.lock().map_err(database_error)?;
    conn.execute("UPDATE integration_outbox SET status='pending', updated_at=datetime('now') WHERE status='sending'", [])
        .map_err(database_error)?;
    Ok(())
}

fn flush_outbox_unlocked(db: &Db) -> Result<(), String> {
    let token = super::get_token()?.ok_or("No Todoist token configured")?;
    let entries: Vec<(i64, String, String)> = {
        let conn = db.0.lock().map_err(database_error)?;
        let result = conn.prepare("SELECT id,command_uuid,payload_json FROM integration_outbox WHERE status IN ('pending','failed') ORDER BY id").map_err(database_error)?.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).map_err(database_error)?.collect::<Result<_,_>>().map_err(database_error)?;
        result
    };
    if entries.is_empty() {
        return Ok(());
    }
    let commands: Vec<serde_json::Value> = entries
        .iter()
        .map(|(_, _, payload)| {
            serde_json::from_str(payload).map_err(|_| "Invalid queued Todoist command".to_string())
        })
        .collect::<Result<_, _>>()?;
    {
        let conn = db.0.lock().map_err(database_error)?;
        for (id, _, _) in &entries {
            conn.execute("UPDATE integration_outbox SET status='sending',attempts=attempts+1,updated_at=datetime('now') WHERE id=?1", [id]).map_err(database_error)?;
        }
    }
    let client = TodoistClient::new(token).map_err(|e| e.to_string())?;
    match client.complete_tasks(&commands) {
        Ok(()) => {
            let conn = db.0.lock().map_err(database_error)?;
            for (id, _, _) in &entries {
                conn.execute("UPDATE integration_outbox SET status='completed',updated_at=datetime('now') WHERE id=?1", [id]).map_err(database_error)?;
            }
            Ok(())
        }
        Err(error) => {
            let conn = db.0.lock().map_err(database_error)?;
            let message = error.to_string();
            for (id, _, _) in &entries {
                conn.execute("UPDATE integration_outbox SET status='failed',last_error=?1,updated_at=datetime('now') WHERE id=?2", rusqlite::params![message,id]).map_err(database_error)?;
            }
            Err(message)
        }
    }
}

fn sync_with(
    db: &Db,
    fetch: impl FnOnce(&str) -> Result<Vec<RemoteSync>, String>,
) -> Result<SyncResult, String> {
    let cursor = {
        let conn = db.0.lock().map_err(database_error)?;
        conn.query_row(
            "SELECT value FROM sync_metadata WHERE key='todoist_cursor'",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?
        .unwrap_or_else(|| "*".into())
    }; // No DB guard crosses the network boundary.
    let snapshots = fetch(&cursor)?;
    let conn = db.0.lock().map_err(database_error)?;
    apply(&conn, &snapshots).map_err(database_error)
}

pub(crate) fn apply(conn: &Connection, snapshots: &[RemoteSync]) -> rusqlite::Result<SyncResult> {
    let tx = conn.unchecked_transaction()?;
    let mut projects_synced = 0;
    for snapshot in snapshots {
        if snapshot.full_sync {
            tx.execute("DELETE FROM todoist_task_cache", [])?;
            // Absence is NOT proof of completion/deletion. Keep history and hide unknown work.
            tx.execute("UPDATE tasks SET external_state='unknown' WHERE source='todoist' AND external_state='active'", [])?;
        }
        for project in &snapshot.projects {
            if project.is_deleted {
                tx.execute(
                    "UPDATE todoist_projects SET class_id=NULL WHERE todoist_id=?1",
                    [&project.id],
                )?;
            } else {
                tx.execute("INSERT INTO todoist_projects(todoist_id,name,synced_at) VALUES(?1,?2,datetime('now')) ON CONFLICT(todoist_id) DO UPDATE SET name=excluded.name,synced_at=excluded.synced_at", rusqlite::params![project.id,project.name])?;
                projects_synced += 1;
            }
        }
        for task in &snapshot.items {
            if task.is_deleted || task.checked {
                let state = if task.is_deleted {
                    "deleted"
                } else {
                    "completed"
                };
                tx.execute("UPDATE tasks SET external_state=?1, status=CASE WHEN ?1='completed' THEN 'completed' ELSE status END, completed_at=?2, updated_at=datetime('now') WHERE source='todoist' AND external_id=?3", rusqlite::params![state,task.completed_at,task.id])?;
                tx.execute(
                    "DELETE FROM todoist_task_cache WHERE todoist_id=?1",
                    [&task.id],
                )?;
            } else {
                if task.content.trim().is_empty() || task.project_id.is_empty() {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                let payload =
                    serde_json::to_string(task).map_err(|_| rusqlite::Error::InvalidQuery)?;
                tx.execute("INSERT INTO todoist_task_cache(todoist_id,payload) VALUES(?1,?2) ON CONFLICT(todoist_id) DO UPDATE SET payload=excluded.payload", rusqlite::params![task.id,payload])?;
            }
        }
        tx.execute("INSERT INTO sync_metadata(key,value) VALUES('todoist_cursor',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [&snapshot.sync_token])?;
    }
    let tasks_synced = reconcile_cached(&tx)?;
    tx.execute("INSERT INTO sync_metadata(key,value) VALUES('todoist_last_sync',datetime('now')) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [])?;
    tx.commit()?;
    Ok(SyncResult {
        projects_synced,
        tasks_synced,
    })
}

pub(crate) fn reconcile_cached(conn: &Connection) -> rusqlite::Result<usize> {
    let payloads = conn
        .prepare("SELECT payload FROM todoist_task_cache")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut tasks = HashMap::new();
    for payload in payloads {
        let task: TodoistTask =
            serde_json::from_str(&payload).map_err(|_| rusqlite::Error::InvalidQuery)?;
        tasks.insert(task.id.clone(), task);
    }
    // Clear old edges before moving projects/classes. Rebuild only after every row exists.
    conn.execute(
        "UPDATE tasks SET parent_task_id=NULL WHERE source='todoist' AND external_id IN (SELECT todoist_id FROM todoist_task_cache)",
        [],
    )?;
    let mut imported = HashSet::new();
    for task in tasks.values() {
        let class_id: Option<i64> = conn
            .query_row(
                "SELECT class_id FROM todoist_projects WHERE todoist_id=?1",
                [&task.project_id],
                |r| r.get(0),
            )
            .optional()?
            .flatten()
            .or_else(|| {
                task.labels.iter().find_map(|label| {
                    conn.query_row(
                        "SELECT id FROM classes WHERE active=1 AND (course_code=?1 OR name=?1)",
                        [label],
                        |r| r.get(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
                })
            });
        let Some(class_id) = class_id else {
            continue;
        };
        let due = task
            .due
            .as_ref()
            .map(|d| d.datetime.as_ref().unwrap_or(&d.date));
        conn.execute("INSERT INTO tasks(class_id,title,description,priority,due_at,source,external_id,todoist_project_id,external_state) VALUES(?1,?2,?3,?4,?5,'todoist',?6,?7,'active') ON CONFLICT(source,external_id) DO UPDATE SET class_id=excluded.class_id,title=excluded.title,description=excluded.description,priority=excluded.priority,due_at=excluded.due_at,todoist_project_id=excluded.todoist_project_id,external_state='active',status=CASE WHEN tasks.status='completed' THEN 'not_started' ELSE tasks.status END,completed_at=NULL,updated_at=datetime('now')", rusqlite::params![class_id,task.content,task.description,task.priority,due,task.id,task.project_id])?;
        imported.insert(task.id.clone());
    }
    for id in &imported {
        let task = &tasks[id];
        if let Some(parent) = &task.parent_id {
            // Reject malformed cycles rather than hanging recursive Today/analytics queries.
            let mut seen = HashSet::from([id.as_str()]);
            let mut ancestor = Some(parent.as_str());
            while let Some(pid) = ancestor {
                if !seen.insert(pid) {
                    return Err(rusqlite::Error::InvalidQuery);
                }
                ancestor = tasks.get(pid).and_then(|t| t.parent_id.as_deref());
            }
            if imported.contains(parent) && tasks[parent].project_id == task.project_id {
                conn.execute("UPDATE tasks SET parent_task_id=(SELECT id FROM tasks WHERE source='todoist' AND external_id=?1) WHERE source='todoist' AND external_id=?2", rusqlite::params![parent,id])?;
            }
        }
    }
    Ok(imported.len())
}

pub(crate) fn map_project(conn: &Connection, id: &str, class: Option<i64>) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    if tx.execute(
        "UPDATE todoist_projects SET class_id=?1 WHERE todoist_id=?2",
        rusqlite::params![class, id],
    )? != 1
    {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    reconcile_cached(&tx)?;
    tx.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_connection;
    fn snapshot(items: serde_json::Value, full: bool) -> RemoteSync {
        serde_json::from_value(serde_json::json!({"projects":[{"id":"p","name":"Course"}],"items":items,"sync_token":"next","full_sync":full})).unwrap()
    }
    fn task(id: &str, parent: Option<&str>) -> serde_json::Value {
        serde_json::json!({"id":id,"project_id":"p","parent_id":parent,"content":id})
    }
    #[test]
    fn labels_match_classes_when_project_is_unmapped() {
        let conn = test_connection();
        conn.execute(
            "INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST 1000','Fall')",
            [],
        )
        .unwrap();
        apply(&conn, &[snapshot(serde_json::json!([{"id":"tagged","project_id":"p","parent_id":null,"content":"Tagged","labels":["LGST 1000"]}]), true)]).unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM tasks WHERE class_id=1", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    fn seed(conn: &Connection) {
        conn.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall'),(2,'CRIM','Fall'); INSERT INTO todoist_projects(todoist_id,name,class_id) VALUES('p','Course',1);").unwrap();
    }
    #[test]
    fn initial_sync_and_idempotent_deep_tree() {
        let conn = test_connection();
        seed(&conn);
        for _ in 0..2 {
            apply(
                &conn,
                &[snapshot(
                    serde_json::json!([
                        task("d", Some("c")),
                        task("c", Some("b")),
                        task("b", Some("a")),
                        task("a", None)
                    ]),
                    true,
                )],
            )
            .unwrap();
        }
        assert_eq!(
            conn.query_row("SELECT count(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            4
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM tasks WHERE parent_task_id IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            3
        );
    }
    #[test]
    fn unmapped_then_mapped_then_moved_and_unmapped() {
        let conn = test_connection();
        seed(&conn);
        map_project(&conn, "p", None).unwrap();
        apply(
            &conn,
            &[snapshot(serde_json::json!([task("a", None)]), true)],
        )
        .unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        map_project(&conn, "p", Some(1)).unwrap();
        let id: i64 = conn
            .query_row("SELECT id FROM tasks", [], |r| r.get(0))
            .unwrap();
        map_project(&conn, "p", Some(2)).unwrap();
        assert_eq!(
            conn.query_row("SELECT class_id FROM tasks WHERE id=?1", [id], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        map_project(&conn, "p", None).unwrap();
        assert_eq!(
            conn.query_row("SELECT class_id FROM todoist_projects", [], |r| r
                .get::<_, Option<i64>>(0))
                .unwrap(),
            None
        );
    }
    #[test]
    fn rename_due_completion_deletion_preserve_history() {
        let conn = test_connection();
        seed(&conn);
        apply(
            &conn,
            &[snapshot(
                serde_json::json!([task("a", None), task("b", None)]),
                true,
            )],
        )
        .unwrap();
        conn.execute_batch("INSERT INTO time_sessions(task_id,start_ts,end_ts,final_duration_seconds) SELECT id,'2026-09-15','2026-09-15',60 FROM tasks").unwrap();
        let mut a = task("a", None);
        a["content"] = "Renamed".into();
        a["due"] = serde_json::json!({"date":"2026-09-20"});
        apply(&conn, &[snapshot(serde_json::json!([a]), false)]).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT title || due_at FROM tasks WHERE external_id='a'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "Renamed2026-09-20"
        );
        apply(
            &conn,
            &[snapshot(
                serde_json::json!([{"id":"a","checked":true},{"id":"b","is_deleted":true}]),
                false,
            )],
        )
        .unwrap();
        assert_eq!(
            conn.query_row("SELECT status FROM tasks WHERE external_id='a'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            "completed"
        );
        assert_eq!(
            conn.query_row(
                "SELECT external_state FROM tasks WHERE external_id='b'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "deleted"
        );
        assert_eq!(
            conn.query_row(
                "SELECT sum(final_duration_seconds) FROM time_sessions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            120
        );
    }
    #[test]
    fn local_failure_rolls_back_everything_including_cursor() {
        let conn = test_connection();
        seed(&conn);
        conn.execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON tasks BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
        assert!(apply(
            &conn,
            &[snapshot(serde_json::json!([task("a", None)]), true)]
        )
        .is_err());
        for table in ["tasks", "sync_metadata", "todoist_task_cache"] {
            assert_eq!(
                conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                    .unwrap(),
                0
            );
        }
    }
    #[test]
    fn network_failure_and_slow_fetch_leave_database_available() {
        let db = Db(Mutex::new(test_connection()));
        let result = sync_with(&db, |_| {
            let conn = db.0.try_lock().expect("network must not hold DB");
            seed(&conn);
            conn.execute("INSERT INTO tasks(class_id,title) VALUES(1,'Local')", [])
                .unwrap();
            let id = conn.last_insert_rowid();
            crate::timer::core_start_timer(&conn, id).unwrap();
            crate::timer::core_finish_timer(&conn).unwrap();
            Err("offline".into())
        });
        assert!(result.is_err());
    }
    #[test]
    fn malformed_cycle_fails_atomically() {
        let conn = test_connection();
        seed(&conn);
        assert!(apply(
            &conn,
            &[snapshot(
                serde_json::json!([task("a", Some("b")), task("b", Some("a"))]),
                true
            )]
        )
        .is_err());
    }
    #[test]
    fn full_sync_absence_is_unknown_not_false_completion() {
        let conn = test_connection();
        seed(&conn);
        apply(
            &conn,
            &[snapshot(serde_json::json!([task("a", None)]), true)],
        )
        .unwrap();
        apply(&conn, &[snapshot(serde_json::json!([]), true)]).unwrap();
        assert_eq!(
            conn.query_row("SELECT external_state FROM tasks", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "unknown"
        );
    }
    #[test]
    fn completion_is_persisted_with_stable_command_and_local_status() {
        let conn = test_connection();
        seed(&conn);
        conn.execute("INSERT INTO tasks(id,class_id,title,source,external_id,todoist_project_id) VALUES(1,1,'Remote','todoist','remote-1','p')", []).unwrap();
        queue_completion(&conn, 1).unwrap();
        queue_completion(&conn, 1).unwrap();
        assert_eq!(
            conn.query_row("SELECT status FROM tasks WHERE id=1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "completed"
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM integration_outbox", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        let payload: String = conn
            .query_row("SELECT payload_json FROM integration_outbox", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(payload.contains("item_complete"));
        assert!(payload.contains("remote-1"));
    }
}
