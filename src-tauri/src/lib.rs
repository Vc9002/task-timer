mod commands;
mod db;
mod timer;
mod todoist;

use commands::analytics::{
    get_analytics_month, get_analytics_today, get_analytics_week, get_day_view, get_task_history,
};
use commands::classes::{archive_class, create_class, list_classes, update_class};
use commands::tasks::{
    create_task, delete_task, list_tasks_for_class, set_task_status, update_task,
};
use commands::today::get_today;
use commands::todoist::{
    disconnect_todoist, get_todoist_status, list_todoist_project_mappings, map_todoist_project,
    set_todoist_token, sync_todoist_now,
};
use db::Db;
use std::sync::Mutex;
use tauri::Manager;
use timer::{
    cancel_timer, edit_session_duration, finish_timer, get_active_session, list_sessions_for_task,
    pause_timer, resume_timer, start_timer,
};

/// How often to check for a background Todoist sync while the app is open.
/// Deliberately conservative — no sync at all while idle/minimized.
const BACKGROUND_SYNC_INTERVAL_SECS: u64 = 15 * 60;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let conn = db::open(&app_data_dir).expect("failed to open database");
            app.manage(Db(Mutex::new(conn)));

            let app_handle = app.handle().clone();
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(
                    BACKGROUND_SYNC_INTERVAL_SECS,
                ));
                if let (Ok(Some(token)), Some(db)) =
                    (todoist::get_token(), app_handle.try_state::<Db>())
                {
                    if let Ok(conn) = db.0.lock() {
                        let _ = todoist::sync::sync_now(&conn, &token);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_classes,
            create_class,
            update_class,
            archive_class,
            list_tasks_for_class,
            create_task,
            update_task,
            set_task_status,
            delete_task,
            get_today,
            get_active_session,
            start_timer,
            pause_timer,
            resume_timer,
            finish_timer,
            cancel_timer,
            list_sessions_for_task,
            edit_session_duration,
            get_analytics_today,
            get_analytics_week,
            get_analytics_month,
            get_day_view,
            get_task_history,
            get_todoist_status,
            set_todoist_token,
            disconnect_todoist,
            list_todoist_project_mappings,
            map_todoist_project,
            sync_todoist_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
