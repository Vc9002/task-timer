use super::client::{TodoistApiError, TodoistClient, TodoistTask};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub projects_synced: usize,
    pub tasks_synced: usize,
}

/// Read-only sync (Phase 1 of Todoist integration): pulls projects and tasks,
/// upserts by (source='todoist', external_id) so re-running never duplicates.
pub fn sync_now(conn: &Connection, token: &str) -> Result<SyncResult, TodoistApiError> {
    let client = TodoistClient::new(token.to_string());

    let projects = client.fetch_all_projects()?;
    for project in &projects {
        conn.execute(
            "INSERT INTO todoist_projects (todoist_id, name, synced_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(todoist_id) DO UPDATE SET name = excluded.name, synced_at = excluded.synced_at",
            rusqlite::params![project.id, project.name],
        )
        .map_err(|e| TodoistApiError::Network(e.to_string()))?;
    }

    let tasks = client.fetch_all_tasks()?;

    // Parents must be upserted before children so parent_task_id can resolve.
    let mut ordered: Vec<&TodoistTask> = tasks.iter().collect();
    ordered.sort_by_key(|t| t.parent_id.is_some());

    let mut synced = 0usize;
    for task in ordered {
        if upsert_task(conn, task).is_ok() {
            synced += 1;
        }
    }

    Ok(SyncResult {
        projects_synced: projects.len(),
        tasks_synced: synced,
    })
}

pub(crate) fn upsert_task(conn: &Connection, task: &TodoistTask) -> rusqlite::Result<()> {
    let class_id: Option<i64> = conn
        .query_row(
            "SELECT class_id FROM todoist_projects WHERE todoist_id = ?1",
            [&task.project_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let Some(class_id) = class_id else {
        // Project not mapped to a class yet — skip until the user maps it.
        return Ok(());
    };

    let parent_task_id: Option<i64> = match &task.parent_id {
        Some(pid) => conn
            .query_row(
                "SELECT id FROM tasks WHERE source = 'todoist' AND external_id = ?1",
                [pid],
                |r| r.get(0),
            )
            .ok(),
        None => None,
    };

    let due_at = task
        .due
        .as_ref()
        .map(|d| d.datetime.clone().unwrap_or_else(|| d.date.clone()));

    conn.execute(
        "INSERT INTO tasks (class_id, parent_task_id, title, description, priority, due_at,
             source, external_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'todoist', ?7)
         ON CONFLICT(source, external_id) DO UPDATE SET
             class_id = excluded.class_id,
             parent_task_id = excluded.parent_task_id,
             title = excluded.title,
             description = excluded.description,
             priority = excluded.priority,
             due_at = excluded.due_at,
             updated_at = datetime('now')",
        rusqlite::params![
            class_id,
            parent_task_id,
            task.content,
            task.description,
            task.priority,
            due_at,
            task.id,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::client::TodoistDue;
    use super::*;
    use crate::db;

    fn test_conn() -> Connection {
        // A counter, not just a timestamp: parallel test threads share a pid
        // and can share a nanosecond tick, so timestamp-only names can collide.
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut dir = std::env::temp_dir();
        dir.push(format!("task-timer-sync-test-{}-{}", std::process::id(), n));
        db::open(&dir).expect("open test db")
    }

    fn seed_mapped_class(conn: &Connection, todoist_project_id: &str) -> i64 {
        conn.execute(
            "INSERT INTO classes (course_code, semester, todoist_project_id)
             VALUES ('TEST 1000', 'Fall 2026', ?1)",
            [todoist_project_id],
        )
        .unwrap();
        let class_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO todoist_projects (todoist_id, name, class_id) VALUES (?1, 'Test Project', ?2)",
            rusqlite::params![todoist_project_id, class_id],
        )
        .unwrap();
        class_id
    }

    fn sample_task(id: &str, parent_id: Option<&str>) -> TodoistTask {
        TodoistTask {
            id: id.to_string(),
            project_id: "proj1".to_string(),
            parent_id: parent_id.map(|s| s.to_string()),
            content: "Read Chapter 6".to_string(),
            description: None,
            due: Some(TodoistDue {
                date: "2026-09-20".to_string(),
                datetime: None,
            }),
            priority: Some(1),
        }
    }

    #[test]
    fn upsert_is_idempotent_running_sync_twice_does_not_duplicate() {
        let conn = test_conn();
        seed_mapped_class(&conn, "proj1");
        let task = sample_task("todoist-1", None);

        upsert_task(&conn, &task).unwrap();
        upsert_task(&conn, &task).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tasks WHERE source = 'todoist' AND external_id = ?1",
                ["todoist-1"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "re-syncing the same task must not duplicate it");
    }

    #[test]
    fn upsert_skips_tasks_from_unmapped_projects() {
        let conn = test_conn();
        // No class mapped to "proj1" — task must be skipped, not errored.
        let task = sample_task("todoist-1", None);
        upsert_task(&conn, &task).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn upsert_preserves_parent_child_relationship() {
        let conn = test_conn();
        seed_mapped_class(&conn, "proj1");
        let parent = sample_task("todoist-parent", None);
        let child = sample_task("todoist-child", Some("todoist-parent"));

        upsert_task(&conn, &parent).unwrap();
        upsert_task(&conn, &child).unwrap();

        let parent_task_id: Option<i64> = conn
            .query_row(
                "SELECT parent_task_id FROM tasks WHERE external_id = 'todoist-child'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let expected_parent_id: i64 = conn
            .query_row(
                "SELECT id FROM tasks WHERE external_id = 'todoist-parent'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(parent_task_id, Some(expected_parent_id));
    }

    #[test]
    fn upsert_updates_renamed_task_content_without_duplicating() {
        let conn = test_conn();
        seed_mapped_class(&conn, "proj1");
        let mut task = sample_task("todoist-1", None);
        upsert_task(&conn, &task).unwrap();

        task.content = "Renamed title".to_string();
        upsert_task(&conn, &task).unwrap();

        let (count, title): (i64, String) = conn
            .query_row(
                "SELECT COUNT(*), MAX(title) FROM tasks WHERE external_id = 'todoist-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(title, "Renamed title");
    }
}
