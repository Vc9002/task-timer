use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

/// Migrations in order. Each one runs exactly once, tracked in `schema_migrations`.
const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_init",
    include_str!("migrations/0001_init.sql"),
)];

pub fn open(app_data_dir: &Path) -> rusqlite::Result<Connection> {
    std::fs::create_dir_all(app_data_dir).expect("failed to create app data dir");
    let db_path = app_data_dir.join("task-timer.sqlite");
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;",
    )?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            name TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    for (name, sql) in MIGRATIONS {
        let already_applied: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = ?1)",
                [name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if already_applied {
            continue;
        }

        conn.execute_batch(sql)?;
        conn.execute(
            "INSERT INTO schema_migrations (name) VALUES (?1)",
            [name],
        )?;
    }

    Ok(())
}
