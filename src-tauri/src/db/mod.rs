use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

/// Migrations in order. Each one runs exactly once, tracked in `schema_migrations`.
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_init", include_str!("migrations/0001_init.sql")),
    (
        "0002_sync_safety",
        include_str!("migrations/0002_sync_safety.sql"),
    ),
    (
        "0003_session_notifications",
        include_str!("migrations/0003_session_notifications.sql"),
    ),
    (
        "0004_week_planning",
        include_str!("migrations/0004_week_planning.sql"),
    ),
    (
        "0005_recurring_tasks",
        include_str!("migrations/0005_recurring_tasks.sql"),
    ),
    (
        "0006_todoist_outbox",
        include_str!("migrations/0006_todoist_outbox.sql"),
    ),
    (
        "0007_productivity_workflow",
        include_str!("migrations/0007_productivity_workflow.sql"),
    ),
    (
        "0008_class_timers",
        include_str!("migrations/0008_class_timers.sql"),
    ),
    (
        "0009_study_blocks",
        include_str!("migrations/0009_study_blocks.sql"),
    ),
    (
        "0010_task_milestones",
        include_str!("migrations/0010_task_milestones.sql"),
    ),
    (
        "0011_task_templates",
        include_str!("migrations/0011_task_templates.sql"),
    ),
    ("0012_exams", include_str!("migrations/0012_exams.sql")),
    (
        "0013_task_notes",
        include_str!("migrations/0013_task_notes.sql"),
    ),
    (
        "0014_task_dependencies",
        include_str!("migrations/0014_task_dependencies.sql"),
    ),
    (
        "0015_session_last_resumed",
        include_str!("migrations/0015_session_last_resumed.sql"),
    ),
];

pub fn open(app_data_dir: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(app_data_dir).expect("failed to create app data dir");
    let db_path = app_data_dir.join("task-timer.sqlite");
    let conn = Connection::open(db_path)?;
    configure(&conn)?;
    run_migrations(&conn)?;
    Ok(conn)
}

/// WAL with synchronous=NORMAL is SQLite's recommended pairing: commits stop
/// forcing an fsync each time, and the database stays consistent after a crash
/// (only the last few commits can be lost on sudden power loss, never corrupted).
fn configure(conn: &Connection) -> rusqlite::Result<()> {
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA foreign_keys = ON;",
    )
}

fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    for (name, sql) in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;

        if already_applied {
            continue;
        }

        apply_migration(conn, name, sql)?;
    }

    Ok(())
}

fn apply_migration(conn: &Connection, name: &str, sql: &str) -> rusqlite::Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(sql)?;
    tx.execute("INSERT INTO schema_migrations (name) VALUES (?1)", [name])?;
    tx.commit()
}

#[cfg(test)]
pub(crate) fn test_connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
    run_migrations(&conn).unwrap();
    conn
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_upgrade_preserves_history_and_moves_legacy_attempts() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; CREATE TABLE schema_migrations(name TEXT PRIMARY KEY);",
        )
        .unwrap();
        for (name, sql) in &MIGRATIONS[..2] {
            apply_migration(&conn, name, sql).unwrap();
        }
        conn.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall'); INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Read'); INSERT INTO time_sessions(id,task_id,start_ts,end_ts,final_duration_seconds) VALUES(1,1,'2026-09-15','2026-09-16',60); INSERT INTO app_settings(key,value) VALUES('overrun_sent_1_25','attempted'),('overrun_v02','{}');").unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        assert_eq!(
            conn.query_row("SELECT status FROM session_notifications", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "legacy_unknown"
        );
        assert_eq!(
            conn.query_row(
                "SELECT SUM(final_duration_seconds) FROM time_sessions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            60
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM app_settings", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM session_notifications", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
    }
    #[test]
    fn migration_failure_rolls_back_schema_and_version() {
        let conn = test_connection();
        assert!(apply_migration(
            &conn,
            "bad",
            "CREATE TABLE partial(id); INSERT INTO nonexistent VALUES(1);"
        )
        .is_err());
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='partial'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM schema_migrations WHERE name='bad'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        run_migrations(&conn).unwrap();
    }
    #[test]
    fn upgrade_preserves_legacy_mapping_and_history() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch("INSERT INTO classes(id,course_code,semester,todoist_project_id) VALUES(1,'LGST','Fall','p'); INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Read'); INSERT INTO time_sessions(task_id,start_ts,end_ts,final_duration_seconds) VALUES(1,'2026-09-14','2026-09-15',60);").unwrap();
        conn.execute_batch(include_str!("migrations/0002_sync_safety.sql"))
            .unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT class_id FROM todoist_projects WHERE todoist_id='p'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT final_duration_seconds FROM time_sessions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            60
        );
    }

    #[test]
    fn planning_upgrade_preserves_tasks_and_sessions() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; CREATE TABLE schema_migrations(name TEXT PRIMARY KEY);",
        )
        .unwrap();
        for (name, sql) in &MIGRATIONS[..8] {
            apply_migration(&conn, name, sql).unwrap();
        }
        conn.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST 1000','Fall 2026'); INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Brief'); INSERT INTO time_sessions(task_id,start_ts,end_ts,final_duration_seconds) VALUES(1,'2026-09-15','2026-09-15 01:00',3600);")
            .unwrap();
        run_migrations(&conn).unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT final_duration_seconds FROM time_sessions",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            3600
        );
        for table in ["study_blocks", "task_milestones", "task_templates"] {
            assert_eq!(
                conn.query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
        }
    }

    #[test]
    fn open_configures_wal_and_foreign_keys() {
        let dir = std::env::temp_dir().join(format!("tt-db-open-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let conn = open(&dir).unwrap();
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        let sync: i64 = conn
            .query_row("PRAGMA synchronous", [], |r| r.get(0))
            .unwrap();
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        assert_eq!(sync, 1, "NORMAL");
        assert_eq!(fk, 1);
        drop(conn);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn last_resumed_upgrade_preserves_sessions() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; CREATE TABLE schema_migrations(name TEXT PRIMARY KEY);",
        )
        .unwrap();
        for (name, sql) in &MIGRATIONS[..14] {
            apply_migration(&conn, name, sql).unwrap();
        }
        conn.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall'); INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Read'); INSERT INTO time_sessions(id,task_id,start_ts,accumulated_pause_seconds) VALUES(1,1,'2026-09-15 10:00:00',120);").unwrap();
        run_migrations(&conn).unwrap();
        let (pause, resumed): (i64, Option<String>) = conn
            .query_row(
                "SELECT accumulated_pause_seconds, last_resumed_ts FROM time_sessions WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(pause, 120);
        assert_eq!(resumed, None);
    }
}
