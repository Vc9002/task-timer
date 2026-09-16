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
#[tauri::command]
pub async fn export_data(app: AppHandle, format: String) -> Result<bool, String> {
    if !["csv", "tasks", "json"].contains(&format.as_str()) {
        return Err("Choose CSV or JSON export.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
    let extension = if format == "json" { "json" } else { "csv" };
    let Some(file) = app
        .dialog()
        .file()
        .add_filter("TaskTimer export", &[extension])
        .set_file_name(if format != "json" {
            "tasktimer-export.csv"
        } else {
            "tasktimer-export.json"
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
}
