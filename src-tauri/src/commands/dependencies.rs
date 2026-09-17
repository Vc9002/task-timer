use crate::db::Db;
use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DependencySummary {
    pub id: i64,
    pub title: String,
    pub status: String,
}

fn row_to_summary(row: &rusqlite::Row) -> rusqlite::Result<DependencySummary> {
    Ok(DependencySummary {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
    })
}

#[tauri::command]
pub fn list_task_dependencies(
    db: State<Db>,
    task_id: i64,
) -> Result<Vec<DependencySummary>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT dep.id, dep.title, dep.status FROM task_dependencies td
             JOIN tasks dep ON dep.id = td.depends_on_task_id
             WHERE td.task_id = ?1 ORDER BY dep.id",
        )
        .map_err(|e| e.to_string())?;
    let result = stmt
        .query_map([task_id], row_to_summary)
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    result
}

fn creates_cycle(conn: &Connection, task_id: i64, depends_on_task_id: i64) -> Result<bool, String> {
    conn.query_row(
        "WITH RECURSIVE chain(id) AS (
            SELECT ?2
            UNION
            SELECT td.depends_on_task_id FROM task_dependencies td JOIN chain c ON td.task_id = c.id
         )
         SELECT EXISTS(SELECT 1 FROM chain WHERE id = ?1)",
        [task_id, depends_on_task_id],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_task_dependency(
    db: State<Db>,
    task_id: i64,
    depends_on_task_id: i64,
) -> Result<Vec<DependencySummary>, String> {
    if task_id == depends_on_task_id {
        return Err("A task can't depend on itself.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let both_exist: bool = conn
        .query_row(
            "SELECT (SELECT COUNT(*) FROM tasks WHERE id IN (?1,?2)) = 2",
            [task_id, depends_on_task_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !both_exist {
        return Err("Choose two existing tasks.".into());
    }
    if creates_cycle(&conn, task_id, depends_on_task_id)? {
        return Err("That would create a dependency cycle.".into());
    }
    conn.execute(
        "INSERT OR IGNORE INTO task_dependencies(task_id, depends_on_task_id) VALUES (?1, ?2)",
        [task_id, depends_on_task_id],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    list_task_dependencies(db, task_id)
}

#[tauri::command]
pub fn remove_task_dependency(
    db: State<Db>,
    task_id: i64,
    depends_on_task_id: i64,
) -> Result<Vec<DependencySummary>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM task_dependencies WHERE task_id = ?1 AND depends_on_task_id = ?2",
        [task_id, depends_on_task_id],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    list_task_dependencies(db, task_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Connection {
        let conn = crate::db::test_connection();
        conn.execute_batch(
            "INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall');
             INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Draft');
             INSERT INTO tasks(id,class_id,title) VALUES(2,1,'Outline');
             INSERT INTO tasks(id,class_id,title,status) VALUES(3,1,'Research','completed');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn direct_edge_does_not_falsely_flag_unrelated_pair_as_cyclic() {
        let conn = db();
        conn.execute(
            "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES (2,1)",
            [],
        )
        .unwrap();
        assert!(!creates_cycle(&conn, 3, 2).unwrap());
    }

    #[test]
    fn detects_transitive_cycle() {
        let conn = db();
        conn.execute(
            "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES (1,2)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES (2,3)",
            [],
        )
        .unwrap();
        // 1 depends on 2 depends on 3; making 3 depend on 1 would cycle.
        assert!(creates_cycle(&conn, 3, 1).unwrap());
    }

    #[test]
    fn list_returns_dependency_titles_and_status() {
        let conn = db();
        conn.execute(
            "INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES (1,3)",
            [],
        )
        .unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT dep.id, dep.title, dep.status FROM task_dependencies td
             JOIN tasks dep ON dep.id = td.depends_on_task_id WHERE td.task_id = 1",
            )
            .unwrap();
        let rows: Vec<DependencySummary> = stmt
            .query_map([], row_to_summary)
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            rows,
            vec![DependencySummary {
                id: 3,
                title: "Research".into(),
                status: "completed".into()
            }]
        );
    }
}
