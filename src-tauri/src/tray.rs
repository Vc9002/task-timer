use crate::{db::Db, timer};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

pub struct TrayState {
    label: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    finish: MenuItem<tauri::Wry>,
    pending: Mutex<Option<String>>,
}

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn open_action(app: &AppHandle, action: &str) {
    if let Some(state) = app.try_state::<TrayState>() {
        if let Ok(mut pending) = state.pending.lock() {
            *pending = Some(action.to_owned());
        }
    }
    show(app);
    let _ = app.emit("desktop-action", ());
}

#[tauri::command]
pub fn take_desktop_action(app: AppHandle) -> Option<String> {
    app.state::<TrayState>().pending.lock().ok()?.take()
}

pub fn timer_action(app: &AppHandle, action: &str) {
    let result = (|| {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| ())?;
        let active = timer::core_get_active_session(&conn).map_err(|_| ())?;
        let Some(active) = active else {
            return Ok(());
        };
        match action {
            "pause" if active.is_paused => {
                timer::core_resume_timer(&conn).map_err(|_| ())?;
            }
            "pause" => {
                timer::core_pause_timer(&conn).map_err(|_| ())?;
            }
            "finish" => {
                timer::core_finish_timer(&conn).map_err(|_| ())?;
            }
            _ => {}
        }
        Ok::<_, ()>(())
    })();
    if result.is_err() {
        show(app);
        let _ = app.emit(
            "desktop-error",
            "TaskTimer couldn't save the timer change. Try again.",
        );
    }
    changed(app);
}

pub fn changed(app: &AppHandle) {
    crate::reminders::wake(app);
    if let Err(error) = refresh(app) {
        eprintln!("Tray refresh: {error}");
    }
    let _ = app.emit("timer-changed", ());
}

fn refresh(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let Some(state) = app.try_state::<TrayState>() else {
        return Ok(());
    };
    let active = {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| "Timer unavailable")?;
        timer::core_get_active_session(&conn).map_err(|_| "Timer unavailable")?
    };
    state.label.set_text(
        active
            .as_ref()
            .map(|a| format!("{} — {}", a.class_course_code, a.task_title))
            .unwrap_or_else(|| "No active timer".into()),
    )?;
    state
        .pause
        .set_text(if active.as_ref().is_some_and(|a| a.is_paused) {
            "Resume"
        } else {
            "Pause"
        })?;
    state.pause.set_enabled(active.is_some())?;
    state.finish.set_enabled(active.is_some())?;
    Ok(())
}

pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let label = MenuItem::with_id(app, "status", "No active timer", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause", false, None::<&str>)?;
    let finish = MenuItem::with_id(app, "finish", "Finish", false, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start / Switch Task…", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "Quick Add…", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open TaskTimer", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit TaskTimer", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &label, &pause, &finish, &start, &quick, &open, &separator, &quit,
        ],
    )?;
    let mut builder = TrayIconBuilder::with_id("tasktimer")
        .tooltip("TaskTimer")
        .menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id.as_ref() {
            "start" => open_action(app, "start-task"),
            "quick" => open_action(app, "quick-add"),
            "open" => show(app),
            "quit" => app.exit(0),
            "pause" | "finish" => timer_action(app, event.id.as_ref()),
            _ => {}
        })
        .build(app)?;
    app.manage(TrayState {
        label,
        pause,
        finish,
        pending: Mutex::new(None),
    });
    refresh(app)
}

#[derive(Serialize)]
pub struct PickerTask {
    id: i64,
    title: String,
    course_code: String,
    class_name: Option<String>,
}

#[tauri::command]
pub fn list_startable_tasks(db: tauri::State<Db>) -> Result<Vec<PickerTask>, String> {
    let conn = db.0.lock().map_err(|_| "Couldn't load tasks")?;
    let mut stmt = conn.prepare("SELECT t.id,t.title,c.course_code,c.name FROM tasks t JOIN classes c ON c.id=t.class_id WHERE c.active=1 AND t.status!='completed' AND t.external_state='active' AND (t.source='local' OR EXISTS(SELECT 1 FROM todoist_projects p WHERE p.todoist_id=t.todoist_project_id AND p.class_id=t.class_id)) ORDER BY c.course_code,t.title,t.id").map_err(|_| "Couldn't load tasks")?;
    let result = stmt
        .query_map([], |r| {
            Ok(PickerTask {
                id: r.get(0)?,
                title: r.get(1)?,
                course_code: r.get(2)?,
                class_name: r.get(3)?,
            })
        })
        .map_err(|_| "Couldn't load tasks")?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Couldn't load tasks".into());
    result
}
