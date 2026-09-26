use crate::{db::Db, timer};
use serde::Serialize;
use std::{
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

pub struct QuickCaptureState {
    pub shown_at: Mutex<Instant>,
}

/// A window can report a spurious focus-loss right as it's first shown (a
/// race between show() and set_focus() completing), which would otherwise
/// hide the popover immediately after opening it. Ignore blur events within
/// this window after showing.
const BLUR_GRACE_PERIOD: Duration = Duration::from_millis(400);

/// Whether a `Focused(false)` event on the quick-capture window this soon
/// after showing it should be ignored as spurious.
pub fn quick_capture_blur_is_spurious(app: &AppHandle) -> bool {
    app.try_state::<QuickCaptureState>()
        .map(|state| {
            state
                .shown_at
                .lock()
                .map(|shown_at| shown_at.elapsed() < BLUR_GRACE_PERIOD)
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

pub struct TrayState {
    label: MenuItem<tauri::Wry>,
    pause: MenuItem<tauri::Wry>,
    finish: MenuItem<tauri::Wry>,
    pomodoro: MenuItem<tauri::Wry>,
    pending: Mutex<Option<String>>,
}

/// Pushed by the frontend Pomodoro clock, which is the source of truth for
/// phase/remaining time; the tray just mirrors it.
#[tauri::command]
pub fn set_tray_pomodoro_status(app: AppHandle, text: Option<String>) -> Result<(), String> {
    let Some(state) = app.try_state::<TrayState>() else {
        return Ok(());
    };
    state
        .pomodoro
        .set_text(text.unwrap_or_else(|| "Pomodoro: off".into()))
        .map_err(|e| e.to_string())
}

pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Shows the small always-on-top quick-capture popover without touching the
/// main window, so adding a task doesn't interrupt whatever you're doing.
pub fn show_quick_capture(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("quick-capture") {
        if let Some(state) = app.try_state::<QuickCaptureState>() {
            if let Ok(mut shown_at) = state.shown_at.lock() {
                *shown_at = Instant::now();
            }
        }
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = app.emit("quick-capture-shown", ());
    }
}

#[tauri::command]
pub fn hide_quick_capture(app: AppHandle) {
    if let Some(window) = app.get_webview_window("quick-capture") {
        let _ = window.hide();
    }
}

#[tauri::command]
pub fn toggle_mini_timer(app: AppHandle) -> Result<bool, String> {
    let Some(window) = app.get_webview_window("mini-timer") else {
        return Err("Mini timer window unavailable".into());
    };
    let visible = window.is_visible().map_err(|e| e.to_string())?;
    if visible {
        window.hide().map_err(|e| e.to_string())?;
    } else {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(!visible)
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
            .map(|a| {
                format!(
                    "{} — {} · {}",
                    a.class_course_code,
                    a.task_title,
                    elapsed_label(a.elapsed_seconds)
                )
            })
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

fn elapsed_label(seconds: i64) -> String {
    let total_minutes = seconds.max(0) / 60;
    format!("{}:{:02}", total_minutes / 60, total_minutes % 60)
}

pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let label = MenuItem::with_id(app, "status", "No active timer", false, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause", false, None::<&str>)?;
    let finish = MenuItem::with_id(app, "finish", "Finish", false, None::<&str>)?;
    let pomodoro = MenuItem::with_id(app, "pomodoro", "Pomodoro: off", false, None::<&str>)?;
    let start = MenuItem::with_id(app, "start", "Start / Switch Task…", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "Quick Add…", true, None::<&str>)?;
    let mini = MenuItem::with_id(app, "mini", "Toggle Mini Timer", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open TaskTimer", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit TaskTimer", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &label, &pause, &finish, &pomodoro, &start, &quick, &mini, &open, &separator, &quit,
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
            "quick" => show_quick_capture(app),
            "mini" => {
                let _ = toggle_mini_timer(app.clone());
            }
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
        pomodoro,
        pending: Mutex::new(None),
    });
    refresh(app)?;
    let handle = app.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(60));
        if refresh(&handle).is_err() {
            break;
        }
    });
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::elapsed_label;

    #[test]
    fn elapsed_label_is_compact_and_minute_accurate() {
        assert_eq!(elapsed_label(0), "0:00");
        assert_eq!(elapsed_label(3661), "1:01");
        assert_eq!(elapsed_label(-5), "0:00");
    }
}
