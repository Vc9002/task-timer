use crate::db::Db;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct Exam {
    pub id: i64,
    pub class_id: i64,
    pub course_code: String,
    pub title: String,
    pub exam_date: String,
    pub target_study_minutes: Option<i64>,
    pub tracked_minutes: i64,
    pub study_task_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct NewExam {
    pub class_id: i64,
    pub title: String,
    pub exam_date: String,
    pub target_study_minutes: Option<i64>,
}

fn row_to_exam(row: &rusqlite::Row) -> rusqlite::Result<Exam> {
    Ok(Exam {
        id: row.get(0)?,
        class_id: row.get(1)?,
        course_code: row.get(2)?,
        title: row.get(3)?,
        exam_date: row.get(4)?,
        target_study_minutes: row.get(5)?,
        tracked_minutes: row.get::<_, i64>(6)? / 60,
        study_task_id: row.get(7)?,
    })
}

const EXAM_SELECT: &str = "SELECT e.id,e.class_id,c.course_code,e.title,e.exam_date,e.target_study_minutes,
    COALESCE((SELECT SUM(COALESCE(ts.final_duration_seconds,0)) FROM time_sessions ts WHERE ts.task_id=e.study_task_id AND ts.end_ts IS NOT NULL),0),e.study_task_id
    FROM exams e JOIN classes c ON c.id=e.class_id";

#[tauri::command]
pub fn list_exams(db: State<Db>) -> Result<Vec<Exam>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let result = conn
        .prepare(&format!("{EXAM_SELECT} ORDER BY e.exam_date,e.id"))
        .map_err(|e| e.to_string())?
        .query_map([], row_to_exam)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string());
    result
}

#[tauri::command]
pub fn create_exam(db: State<Db>, input: NewExam) -> Result<Exam, String> {
    if input.title.trim().is_empty() {
        return Err("Exam title cannot be empty.".into());
    }
    NaiveDate::parse_from_str(&input.exam_date, "%Y-%m-%d")
        .map_err(|_| "Choose a valid exam date.".to_string())?;
    if input
        .target_study_minutes
        .is_some_and(|minutes| minutes < 0)
    {
        return Err("Study target cannot be negative.".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let class_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM classes WHERE id=?1 AND active=1)",
            [input.class_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !class_exists {
        return Err("Class not found.".into());
    }
    let task_title = format!("Exam prep · {}", input.title.trim());
    conn.execute("INSERT INTO tasks(class_id,title,due_at,estimated_minutes,task_type) VALUES(?1,?2,?3,?4,'exam')", rusqlite::params![input.class_id,task_title,input.exam_date,input.target_study_minutes]).map_err(|e| e.to_string())?;
    let task_id = conn.last_insert_rowid();
    conn.execute("INSERT INTO exams(class_id,title,exam_date,target_study_minutes,study_task_id) VALUES(?1,?2,?3,?4,?5)", rusqlite::params![input.class_id,input.title.trim(),input.exam_date,input.target_study_minutes,task_id]).map_err(|e| e.to_string())?;
    conn.query_row(
        &format!("{EXAM_SELECT} WHERE e.id=?1"),
        [conn.last_insert_rowid()],
        row_to_exam,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_exam(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM exams WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct SemesterClassSummary {
    pub class_id: i64,
    pub course_code: String,
    pub tracked_seconds: i64,
    pub completed_tasks: i64,
    pub open_tasks: i64,
    pub upcoming_deadlines: i64,
}

#[derive(Debug, Serialize)]
pub struct SemesterDashboard {
    pub semester: String,
    pub tracked_seconds: i64,
    pub completed_tasks: i64,
    pub upcoming_deadlines: i64,
    pub classes: Vec<SemesterClassSummary>,
}

#[tauri::command]
pub fn get_semester_dashboard(
    db: State<Db>,
    semester: String,
) -> Result<SemesterDashboard, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT c.id,c.course_code,
        COALESCE((SELECT SUM(COALESCE(ts.final_duration_seconds,0)) FROM time_sessions ts JOIN tasks st ON st.id=ts.task_id WHERE st.class_id=c.id AND ts.end_ts IS NOT NULL),0),
        COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.class_id=c.id AND t.status='completed'),0),
        COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.class_id=c.id AND t.status!='completed' AND t.is_class_timer=0),0),
        COALESCE((SELECT COUNT(*) FROM tasks t WHERE t.class_id=c.id AND t.status!='completed' AND t.is_class_timer=0 AND t.due_at IS NOT NULL AND date(t.due_at)>=date('now','localtime') AND date(t.due_at)<=date('now','localtime','+14 days')),0)
        FROM classes c WHERE c.semester=?1 AND c.active=1 ORDER BY c.course_code").map_err(|e| e.to_string())?;
    let classes = stmt
        .query_map([&semester], |row| {
            Ok(SemesterClassSummary {
                class_id: row.get(0)?,
                course_code: row.get(1)?,
                tracked_seconds: row.get(2)?,
                completed_tasks: row.get(3)?,
                open_tasks: row.get(4)?,
                upcoming_deadlines: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let tracked_seconds = classes.iter().map(|item| item.tracked_seconds).sum();
    let completed_tasks = classes.iter().map(|item| item.completed_tasks).sum();
    let upcoming_deadlines = classes.iter().map(|item| item.upcoming_deadlines).sum();
    Ok(SemesterDashboard {
        semester,
        tracked_seconds,
        completed_tasks,
        upcoming_deadlines,
        classes,
    })
}
