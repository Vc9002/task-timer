use crate::db::Db;
use chrono::{Local, NaiveDate};
use rusqlite::OptionalExtension;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const POLL_INTERVAL: Duration = Duration::from_secs(60 * 60);
const WARN_DAYS: i64 = 3;

struct DueExam {
    id: i64,
    title: String,
    course_code: String,
    exam_date: String,
    target_study_minutes: Option<i64>,
    tracked_minutes: i64,
}

fn due_soon_exams(conn: &rusqlite::Connection, today: NaiveDate) -> rusqlite::Result<Vec<DueExam>> {
    let sql = "SELECT e.id,e.title,c.course_code,e.exam_date,e.target_study_minutes,
        COALESCE((SELECT SUM(COALESCE(ts.final_duration_seconds,0)) FROM time_sessions ts WHERE ts.task_id=e.study_task_id AND ts.end_ts IS NOT NULL),0)/60
        FROM exams e JOIN classes c ON c.id=e.class_id
        WHERE date(e.exam_date) >= date(?1)";
    let mut stmt = conn.prepare(sql)?;
    let today_str = today.format("%Y-%m-%d").to_string();
    let rows = stmt
        .query_map([&today_str], |row| {
            Ok(DueExam {
                id: row.get(0)?,
                title: row.get(1)?,
                course_code: row.get(2)?,
                exam_date: row.get(3)?,
                target_study_minutes: row.get(4)?,
                tracked_minutes: row.get(5)?,
            })
        })?
        .collect();
    rows
}

fn already_notified_today(conn: &rusqlite::Connection, key: &str) -> bool {
    conn.query_row("SELECT value FROM app_settings WHERE key=?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .ok()
    .flatten()
    .is_some()
}

fn mark_notified(conn: &rusqlite::Connection, key: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings(key,value) VALUES(?1,'1')
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [key],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn check(app: &AppHandle) -> Result<(), String> {
    if crate::focus_mode::is_active(app) {
        // Deliberately don't mark as notified: re-check once Focus Mode ends.
        return Ok(());
    }
    let db = app.state::<Db>();
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let today = Local::now().date_naive();
    let exams = due_soon_exams(&conn, today).map_err(|e| e.to_string())?;
    let mut to_notify = Vec::new();
    for exam in exams {
        let Ok(exam_date) = NaiveDate::parse_from_str(&exam.exam_date, "%Y-%m-%d") else {
            continue;
        };
        let days_left = (exam_date - today).num_days();
        if !(0..=WARN_DAYS).contains(&days_left) {
            continue;
        }
        let key = format!("exam_countdown_notified_{}_{}", exam.id, today);
        if already_notified_today(&conn, &key) {
            continue;
        }
        to_notify.push((exam, days_left, key));
    }
    drop(conn);
    if to_notify.is_empty() {
        return Ok(());
    }
    let granted = app
        .notification()
        .request_permission()
        .map(|state| state == tauri::plugin::PermissionState::Granted)
        .unwrap_or(false);
    for (exam, days_left, key) in to_notify {
        let day_word = if days_left == 0 {
            "today".to_string()
        } else if days_left == 1 {
            "tomorrow".to_string()
        } else {
            format!("in {days_left} days")
        };
        let body = match exam.target_study_minutes {
            Some(target) if target > 0 => format!(
                "{} ({}) is {}. You've logged {}h of the {}h you planned to study.",
                exam.title,
                exam.course_code,
                day_word,
                exam.tracked_minutes / 60,
                target / 60,
            ),
            _ => format!(
                "{} ({}) is {}. You've logged {}h of study time so far.",
                exam.title,
                exam.course_code,
                day_word,
                exam.tracked_minutes / 60,
            ),
        };
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        mark_notified(&conn, &key)?;
        drop(conn);
        if granted {
            let _ = app
                .notification()
                .builder()
                .title("Exam coming up")
                .body(body)
                .show();
        }
    }
    Ok(())
}

pub fn setup(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || loop {
        if let Err(error) = check(&app) {
            eprintln!("Exam countdown check: {error}");
        }
        std::thread::sleep(POLL_INTERVAL);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_exams_within_warn_window() {
        let conn = crate::db::test_connection();
        conn.execute(
            "INSERT INTO classes(id,course_code,semester) VALUES (1,'CS101','Fall 2026')",
            [],
        )
        .unwrap();
        let today = Local::now().date_naive();
        let soon = today + chrono::Duration::days(2);
        conn.execute(
            "INSERT INTO exams(id,class_id,title,exam_date,target_study_minutes) VALUES (1,1,'Midterm',?1,600)",
            [soon.format("%Y-%m-%d").to_string()],
        )
        .unwrap();
        let exams = due_soon_exams(&conn, today).unwrap();
        assert_eq!(exams.len(), 1);
        assert_eq!(exams[0].title, "Midterm");
    }

    #[test]
    fn notifies_once_per_exam_per_day() {
        let conn = crate::db::test_connection();
        let key = "exam_countdown_notified_1_2026-09-20";
        assert!(!already_notified_today(&conn, key));
        mark_notified(&conn, key).unwrap();
        assert!(already_notified_today(&conn, key));
    }
}
