use crate::db::Db;
use std::fs;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

fn safe(value: String) -> String {
    let v = value.replace('"', "\"\"");
    let v = if v.starts_with(['=', '+', '-', '@']) {
        format!("'{}", v)
    } else {
        v
    };
    format!("\"{}\"", v)
}
#[tauri::command]
pub async fn export_data(app: AppHandle, format: String) -> Result<(), String> {
    if format != "csv" && format != "json" {
        return Err("Choose CSV or JSON export.".into());
    }
    let path = app
        .dialog()
        .file()
        .set_file_name(if format == "csv" {
            "tasktimer-export.csv"
        } else {
            "tasktimer-export.json"
        })
        .blocking_save_file()
        .ok_or("Export cancelled")?
        .into_path()
        .map_err(|_| "Choose a valid export location")?;
    let data = {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| "Couldn't read local data")?;
        if format == "json" {
            let mut tasks = Vec::new();
            let mut stmt=conn.prepare("SELECT id,class_id,parent_task_id,title,description,status,priority,due_at,scheduled_date,estimated_minutes,source,external_id,completed_at FROM tasks ORDER BY id").map_err(|_|"Couldn't read tasks")?;
            let rows=stmt.query_map([],|r| { Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"class_id":r.get::<_,i64>(1)?,"parent_task_id":r.get::<_,Option<i64>>(2)?,"title":r.get::<_,String>(3)?,"description":r.get::<_,Option<String>>(4)?,"status":r.get::<_,String>(5)?,"priority":r.get::<_,Option<i64>>(6)?,"due_at":r.get::<_,Option<String>>(7)?,"scheduled_date":r.get::<_,Option<String>>(8)?,"estimated_minutes":r.get::<_,Option<i64>>(9)?,"source":r.get::<_,String>(10)?,"external_id":r.get::<_,Option<String>>(11)?,"completed_at":r.get::<_,Option<String>>(12)?})) }).map_err(|_|"Couldn't read tasks")?;
            for row in rows {
                tasks.push(row.map_err(|_| "Couldn't read tasks")?);
            }
            let mut sessions = Vec::new();
            let mut stmt=conn.prepare("SELECT id,task_id,start_ts,end_ts,accumulated_pause_seconds,pause_started_ts,final_duration_seconds,edited FROM time_sessions ORDER BY id").map_err(|_|"Couldn't read history")?;
            let rows=stmt.query_map([],|r|Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"task_id":r.get::<_,i64>(1)?,"start_ts":r.get::<_,String>(2)?,"end_ts":r.get::<_,Option<String>>(3)?,"accumulated_pause_seconds":r.get::<_,i64>(4)?,"pause_started_ts":r.get::<_,Option<String>>(5)?,"final_duration_seconds":r.get::<_,Option<i64>>(6)?,"edited":r.get::<_,bool>(7)?}))).map_err(|_|"Couldn't read history")?;
            for row in rows {
                sessions.push(row.map_err(|_| "Couldn't read history")?);
            }
            serde_json::to_vec_pretty(
                &serde_json::json!({"version":1,"tasks":tasks,"time_sessions":sessions}),
            )
            .map_err(|_| "Couldn't encode export")?
        } else {
            let mut out =
                String::from("task_id,task_title,start_time,end_time,duration_seconds,edited\n");
            let mut stmt=conn.prepare("SELECT s.task_id,t.title,s.start_ts,s.end_ts,s.final_duration_seconds,s.edited FROM time_sessions s JOIN tasks t ON t.id=s.task_id ORDER BY s.start_ts").map_err(|_|"Couldn't read history")?;
            let rows = stmt
                .query_map([], |r| {
                    Ok([
                        safe(r.get::<_, i64>(0)?.to_string()),
                        safe(r.get::<_, String>(1)?),
                        safe(r.get::<_, String>(2)?),
                        safe(r.get::<_, Option<String>>(3)?.unwrap_or_default()),
                        safe(r.get::<_, Option<i64>>(4)?.unwrap_or_default().to_string()),
                        safe(r.get::<_, bool>(5)?.to_string()),
                    ]
                    .join(","))
                })
                .map_err(|_| "Couldn't read history")?;
            for row in rows {
                out.push_str(&row.map_err(|_| "Couldn't read history")?);
                out.push('\n');
            }
            out.into_bytes()
        }
    };
    fs::write(path, data)
        .map_err(|_| "TaskTimer couldn't write the export file. Try another location.".to_string())
}
