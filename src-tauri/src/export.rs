use crate::db::Db;
use std::fs;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

fn safe(value: String) -> String {
    let v = value.replace('"', "\"\"");
    let v = if v.trim_start().starts_with(['=', '+', '-', '@']) || v.starts_with(['\t', '\r', '\n'])
    {
        format!("'{}", v)
    } else {
        v
    };
    format!("\"{}\"", v)
}

fn records(conn: &rusqlite::Connection, sql: &str) -> rusqlite::Result<Vec<serde_json::Value>> {
    use rusqlite::types::ValueRef;
    let mut stmt = conn.prepare(sql)?;
    let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let rows = stmt.query_map([], |row| {
        let mut record = serde_json::Map::new();
        for (i, name) in names.iter().enumerate() {
            let value = match row.get_ref(i)? {
                ValueRef::Null => serde_json::Value::Null,
                ValueRef::Integer(n) => n.into(),
                ValueRef::Real(n) => n.into(),
                ValueRef::Text(s) => String::from_utf8_lossy(s).into_owned().into(),
                ValueRef::Blob(_) => return Err(rusqlite::Error::InvalidQuery),
            };
            record.insert(name.clone(), value);
        }
        Ok(record.into())
    })?;
    rows.collect()
}

fn backup(conn: &rusqlite::Connection) -> Result<Vec<u8>, String> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| "Couldn't read local data")?;
    let mut result = serde_json::Map::new();
    result.insert("format_version".into(), 1.into());
    result.insert("exported_at".into(), chrono::Utc::now().to_rfc3339().into());
    // Deliberately exclude keychain, arbitrary settings, cached payloads, and sync cursors.
    for (key, sql) in [
        ("classes", "SELECT id,course_code,name,semester,color,active,created_at,updated_at FROM classes ORDER BY id"),
        ("tasks", "SELECT * FROM tasks ORDER BY id"),
        ("time_sessions", "SELECT * FROM time_sessions ORDER BY id"),
        ("todoist_projects", "SELECT * FROM todoist_projects ORDER BY todoist_id"),
        ("schema_migrations", "SELECT * FROM schema_migrations ORDER BY name"),
    ] {
        result.insert(key.into(), records(&tx,sql).map_err(|_| "Couldn't read backup data")?.into());
    }
    serde_json::to_vec_pretty(&result).map_err(|_| "Couldn't encode backup".into())
}
fn json_to_sql(value: &serde_json::Value) -> rusqlite::types::Value {
    use rusqlite::types::Value as V;
    match value {
        serde_json::Value::Null => V::Null,
        serde_json::Value::Bool(b) => V::Integer(*b as i64),
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(V::Integer)
            .unwrap_or_else(|| V::Real(n.as_f64().unwrap_or(0.0))),
        serde_json::Value::String(s) => V::Text(s.clone()),
        _ => V::Null,
    }
}

/// Replaces every row in `table` with the given backup rows, preserving each
/// row's original id. Column list comes from each row's own JSON keys, so
/// this tolerates schema drift between the backup's app version and this one
/// as long as the columns still exist.
fn restore_table(
    conn: &rusqlite::Connection,
    table: &str,
    rows: &[serde_json::Value],
) -> Result<(), String> {
    conn.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    for row in rows {
        let Some(object) = row.as_object() else {
            continue;
        };
        let columns: Vec<&String> = object.keys().collect();
        let placeholders = vec!["?"; columns.len()].join(",");
        let column_list = columns
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(",");
        let values: Vec<rusqlite::types::Value> =
            columns.iter().map(|c| json_to_sql(&object[*c])).collect();
        conn.execute(
            &format!("INSERT INTO {table}({column_list}) VALUES({placeholders})"),
            rusqlite::params_from_iter(values),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Restores classes, tasks, time_sessions, and todoist_projects from a
/// TaskTimer JSON backup, replacing current rows entirely. Foreign keys are
/// held off during the swap (SQLite doesn't allow toggling them mid-transaction)
/// so restoring the same ids the backup used re-links any planning data
/// (Study Blocks, milestones, exams) that survives; anything left pointing at
/// an id the backup doesn't have is pruned afterward rather than left dangling.
fn restore_backup(conn: &mut rusqlite::Connection, bytes: &[u8]) -> Result<(), String> {
    let backup: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "That file isn't a TaskTimer backup.")?;
    if backup["format_version"].as_i64() != Some(1) {
        return Err("Unsupported backup format.".into());
    }
    let get = |key: &str| -> Vec<serde_json::Value> {
        backup[key].as_array().cloned().unwrap_or_default()
    };
    conn.execute_batch("PRAGMA foreign_keys = OFF")
        .map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    restore_table(&tx, "time_sessions", &get("time_sessions"))?;
    restore_table(&tx, "tasks", &get("tasks"))?;
    restore_table(&tx, "classes", &get("classes"))?;
    restore_table(&tx, "todoist_projects", &get("todoist_projects"))?;
    tx.execute(
        "DELETE FROM study_blocks WHERE task_id NOT IN (SELECT id FROM tasks)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM task_milestones WHERE task_id NOT IN (SELECT id FROM tasks)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE exams SET study_task_id=NULL WHERE study_task_id IS NOT NULL AND study_task_id NOT IN (SELECT id FROM tasks)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM exams WHERE class_id NOT IN (SELECT id FROM classes)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM task_templates WHERE class_id NOT IN (SELECT id FROM classes)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM recurring_task_templates WHERE class_id NOT IN (SELECT id FROM classes)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM session_notifications WHERE session_id NOT IN (SELECT id FROM time_sessions)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM task_dependencies WHERE task_id NOT IN (SELECT id FROM tasks) OR depends_on_task_id NOT IN (SELECT id FROM tasks)",
        [],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON")
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn import_backup(app: AppHandle) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some(file) = app
            .dialog()
            .file()
            .add_filter("TaskTimer backup", &["json"])
            .blocking_pick_file()
        else {
            return Ok(false);
        };
        let path = file.into_path().map_err(|_| "Choose a valid backup file")?;
        let bytes = fs::read(&path).map_err(|_| "Couldn't read that file.".to_string())?;
        let db = app.state::<Db>();
        let mut conn = db.0.lock().map_err(|_| "Couldn't access the database")?;
        restore_backup(&mut conn, &bytes)?;
        Ok(true)
    })
    .await
    .map_err(|_| "Couldn't complete restore")?
}

fn history_csv(conn: &rusqlite::Connection) -> Result<Vec<u8>, String> {
    let headers = "date,course_code,class_name,task_id,task_title,parent_task,start_time,end_time,duration_seconds,duration_minutes,edited,source";
    let mut csv = format!("{headers}\r\n");
    let sql = "SELECT date(s.start_ts,'localtime') AS date,c.course_code,c.name AS class_name,t.id AS task_id,t.title AS task_title,p.title AS parent_task,s.start_ts AS start_time,s.end_ts AS end_time,s.final_duration_seconds AS duration_seconds,s.final_duration_seconds/60.0 AS duration_minutes,s.edited,t.source FROM time_sessions s JOIN tasks t ON t.id=s.task_id JOIN classes c ON c.id=t.class_id LEFT JOIN tasks p ON p.id=t.parent_task_id ORDER BY s.start_ts,s.id";
    for row in records(conn, sql).map_err(|_| "Couldn't read time history")? {
        let cells = headers
            .split(',')
            .map(|key| match &row[key] {
                serde_json::Value::Null => safe(String::new()),
                serde_json::Value::String(s) => safe(s.clone()),
                value => safe(value.to_string()),
            })
            .collect::<Vec<_>>();
        csv.push_str(&cells.join(","));
        csv.push_str("\r\n");
    }
    Ok(csv.into_bytes())
}
/// Escapes text per RFC 5545 section 3.3.11 (comma, semicolon, backslash, newline).
fn ics_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

fn ics_date(value: &str) -> String {
    value.replace('-', "")
}

fn calendar_ics(conn: &rusqlite::Connection) -> Result<Vec<u8>, String> {
    let mut ics = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//TaskTimer//EN\r\nCALSCALE:GREGORIAN\r\n",
    );
    let due_rows = records(
        conn,
        "SELECT t.id,t.title,c.course_code,t.due_at FROM tasks t JOIN classes c ON c.id=t.class_id
         WHERE t.due_at IS NOT NULL AND t.status!='completed' AND c.active=1",
    )
    .map_err(|_| "Couldn't read due dates")?;
    for row in due_rows {
        let Some(due_at) = row["due_at"].as_str() else {
            continue;
        };
        let date = &due_at[..10.min(due_at.len())];
        let course = row["course_code"].as_str().unwrap_or("");
        let title = row["title"].as_str().unwrap_or("");
        ics.push_str(&format!(
            "BEGIN:VEVENT\r\nUID:tasktimer-due-{}@tasktimer\r\nDTSTART;VALUE=DATE:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\n",
            row["id"],
            ics_date(date),
            ics_escape(&format!("{course} — {title}"))
        ));
    }
    let block_rows = records(
        conn,
        "SELECT sb.id,sb.planned_date,sb.planned_start_time,sb.planned_minutes,t.title,c.course_code
         FROM study_blocks sb JOIN tasks t ON t.id=sb.task_id JOIN classes c ON c.id=t.class_id
         WHERE sb.completed=0",
    )
    .map_err(|_| "Couldn't read study blocks")?;
    for row in block_rows {
        let date = row["planned_date"].as_str().unwrap_or("");
        let course = row["course_code"].as_str().unwrap_or("");
        let title = row["title"].as_str().unwrap_or("");
        let minutes = row["planned_minutes"].as_i64().unwrap_or(0);
        let summary = ics_escape(&format!("Study: {course} — {title}"));
        if let Some(start_time) = row["planned_start_time"].as_str() {
            let start = format!("{}T{}00", ics_date(date), start_time.replace(':', ""));
            let end_minutes = minutes;
            let (h, m) = start_time
                .split_once(':')
                .and_then(|(h, m)| Some((h.parse::<i64>().ok()?, m.parse::<i64>().ok()?)))
                .unwrap_or((0, 0));
            let total = h * 60 + m + end_minutes;
            let end = format!(
                "{}T{:02}{:02}00",
                ics_date(date),
                (total / 60) % 24,
                total % 60
            );
            ics.push_str(&format!(
                "BEGIN:VEVENT\r\nUID:tasktimer-block-{}@tasktimer\r\nDTSTART:{}\r\nDTEND:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\n",
                row["id"], start, end, summary
            ));
        } else {
            ics.push_str(&format!(
                "BEGIN:VEVENT\r\nUID:tasktimer-block-{}@tasktimer\r\nDTSTART;VALUE=DATE:{}\r\nSUMMARY:{}\r\nEND:VEVENT\r\n",
                row["id"], ics_date(date), summary
            ));
        }
    }
    ics.push_str("END:VCALENDAR\r\n");
    Ok(ics.into_bytes())
}

#[tauri::command]
pub async fn export_data(app: AppHandle, format: String) -> Result<bool, String> {
    if !["csv", "tasks", "json", "ics"].contains(&format.as_str()) {
        return Err("Choose CSV, JSON, or calendar export.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
    let extension = match format.as_str() { "json" => "json", "ics" => "ics", _ => "csv" };
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("TaskTimer export", &[extension])
        .set_file_name(match format.as_str() {
            "json" => "tasktimer-export.json",
            "ics" => "tasktimer-calendar.ics",
            _ => "tasktimer-export.csv",
        })
        .blocking_save_file() else { return Ok(false); };
    let path = file
        .into_path()
        .map_err(|_| "Choose a valid export location")?;
    if path.extension().and_then(|s| s.to_str()) != Some(extension) {
        return Err(format!("Choose a file ending in .{extension}"));
    }
    let data = {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| "Couldn't read local data")?;
        if format == "json" {
            backup(&conn)?
        } else if format == "ics" {
            calendar_ics(&conn)?
        } else if format == "tasks" {
            let mut csv = String::from("class,task,parent,status,due_date,scheduled_date,estimated_minutes,tracked_minutes,source\r\n");
            for row in records(&conn,"SELECT c.course_code AS class,t.title AS task,p.title AS parent,t.status,t.due_at AS due_date,t.scheduled_date,t.estimated_minutes,(SELECT COALESCE(SUM(final_duration_seconds),0)/60.0 FROM time_sessions WHERE task_id=t.id AND end_ts IS NOT NULL) AS tracked_minutes,t.source FROM tasks t JOIN classes c ON c.id=t.class_id LEFT JOIN tasks p ON p.id=t.parent_task_id ORDER BY t.id").map_err(|_| "Couldn't read tasks")? {
                let cells = ["class","task","parent","status","due_date","scheduled_date","estimated_minutes","tracked_minutes","source"].map(|key| { let v=&row[key]; safe(if v.is_null() { String::new() } else if let Some(s)=v.as_str() { s.into() } else { v.to_string() }) });
                csv.push_str(&cells.join(",")); csv.push_str("\r\n");
            }
            csv.into_bytes()
        } else {
            history_csv(&conn)?
        }
    };
    fs::write(path, data)
        .map_err(|_| "TaskTimer couldn't write the export file. Try another location.".to_string())?;
    Ok(true)
    }).await.map_err(|_| "Couldn't complete export")?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_preserves_class_context_and_excludes_secrets() {
        let conn = crate::db::test_connection();
        conn.execute_batch("INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall'); INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Read'); INSERT INTO time_sessions(task_id,start_ts) VALUES(1,'2026-09-15 10:00:00'); INSERT INTO app_settings(key,value) VALUES('secret','DO_NOT_EXPORT'); INSERT INTO sync_metadata(key,value) VALUES('secret','DO_NOT_EXPORT');").unwrap();
        let bytes = backup(&conn).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("DO_NOT_EXPORT"));
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["classes"][0]["id"], json["tasks"][0]["class_id"]);
        assert!(json["time_sessions"][0]["end_ts"].is_null());
        let csv = String::from_utf8(history_csv(&conn).unwrap()).unwrap();
        assert!(csv.contains("duration_minutes,edited,source"));
        assert!(csv.contains("\"LGST\""));
        assert!(csv.contains("\"\",\"\",\"\",\"0\",\"local\""));
    }
    #[test]
    fn csv_escapes_special_characters_and_formulas() {
        assert_eq!(safe("a,\"b\"\nc".into()), "\"a,\"\"b\"\"\nc\"");
        assert_eq!(safe(" =1+2".into()), "\"' =1+2\"");
    }

    #[test]
    fn restore_replaces_data_and_prunes_orphaned_planning_rows() {
        let mut conn = crate::db::test_connection();
        conn.execute_batch(
            "INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall');
             INSERT INTO tasks(id,class_id,title) VALUES(1,1,'Old task');
             INSERT INTO study_blocks(id,task_id,planned_date,planned_minutes) VALUES(1,1,'2026-09-01',30);
             INSERT INTO time_sessions(id,task_id,start_ts,end_ts,final_duration_seconds) VALUES(1,1,'2026-09-01 10:00:00','2026-09-01 10:30:00',1800);
             INSERT INTO tasks(id,class_id,title) VALUES(3,1,'Another old task');
             INSERT INTO task_dependencies(task_id,depends_on_task_id) VALUES(3,1);",
        )
        .unwrap();
        let backup = serde_json::json!({
            "format_version": 1,
            "classes": [{"id": 1, "course_code": "LGST", "name": null, "semester": "Fall", "color": null, "active": 1, "created_at": "2026-01-01", "updated_at": "2026-01-01"}],
            "tasks": [{"id": 2, "class_id": 1, "parent_task_id": null, "title": "Restored task", "description": null, "status": "not_started", "priority": null, "due_at": null, "scheduled_date": null, "estimated_minutes": null, "source": "local", "external_id": null, "created_at": "2026-01-01", "updated_at": "2026-01-01", "completed_at": null, "todoist_project_id": null, "external_state": "active", "recurring_template_id": null, "occurrence_date": null, "task_type": "assignment", "tags": "[]", "time_budget_minutes": null, "is_class_timer": 0}],
            "time_sessions": [],
            "todoist_projects": [],
        });
        restore_backup(&mut conn, serde_json::to_vec(&backup).unwrap().as_slice()).unwrap();
        let task_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(task_count, 1);
        let title: String = conn
            .query_row("SELECT title FROM tasks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(title, "Restored task");
        let orphaned_blocks: i64 = conn
            .query_row("SELECT COUNT(*) FROM study_blocks", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            orphaned_blocks, 0,
            "block tied to the removed task id 1 must be pruned"
        );
        let orphaned_dependencies: i64 = conn
            .query_row("SELECT COUNT(*) FROM task_dependencies", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            orphaned_dependencies, 0,
            "dependency referencing removed task ids 1 and 3 must be pruned"
        );
    }

    #[test]
    fn restore_rejects_unsupported_format() {
        let mut conn = crate::db::test_connection();
        let bad = serde_json::json!({"format_version": 2});
        assert!(restore_backup(&mut conn, serde_json::to_vec(&bad).unwrap().as_slice()).is_err());
    }

    #[test]
    fn ics_includes_due_dates_and_timed_study_blocks() {
        let conn = crate::db::test_connection();
        conn.execute_batch(
            "INSERT INTO classes(id,course_code,semester) VALUES(1,'LGST','Fall');
             INSERT INTO tasks(id,class_id,title,due_at) VALUES(1,1,'Read Ch.3','2026-09-20T23:59:00');
             INSERT INTO study_blocks(id,task_id,planned_date,planned_start_time,planned_minutes) VALUES(1,1,'2026-09-18','14:30',90);
             INSERT INTO study_blocks(id,task_id,planned_date,planned_minutes) VALUES(2,1,'2026-09-19',60);",
        )
        .unwrap();
        let ics = String::from_utf8(calendar_ics(&conn).unwrap()).unwrap();
        assert!(ics.starts_with("BEGIN:VCALENDAR"));
        assert!(ics.trim_end().ends_with("END:VCALENDAR"));
        assert!(ics.contains("DTSTART;VALUE=DATE:20260920"));
        assert!(ics.contains("LGST"));
        assert!(ics.contains("DTSTART:20260918T143000"));
        assert!(ics.contains("DTEND:20260918T160000"));
        assert!(ics.contains("DTSTART;VALUE=DATE:20260919"));
    }
}
