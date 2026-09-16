mod commands;
mod db;
mod desktop;
mod export;
mod reminders;
mod timer;
mod todoist;
mod tray;

use commands::analytics::{
    get_analytics_month, get_analytics_today, get_analytics_week, get_day_view, get_task_history,
};
use commands::classes::{archive_class, create_class, list_classes, update_class};
use commands::tasks::{
    create_task, delete_task, list_tasks_for_class, schedule_task, set_task_status, update_task,
};
use commands::today::get_today;
use commands::todoist::{
    disconnect_todoist, get_todoist_status, list_todoist_project_mappings, map_todoist_project,
    set_todoist_token, sync_todoist_now,
};
use commands::week::{get_study_capacity, get_week, set_study_capacity};
use db::Db;
use std::sync::Mutex;
use tauri::Manager;
use timer::{
    cancel_timer, edit_session_duration, finish_timer, get_active_session, list_sessions_for_task,
    pause_timer, recover_timer, resume_timer, start_timer, switch_timer,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let autostart = tauri_plugin_autostart::Builder::new().arg("--autostart");
    #[cfg(target_os = "macos")]
    let autostart = autostart.macos_launcher(tauri_plugin_autostart::MacosLauncher::LaunchAgent);
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show(app)
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(autostart.build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let conn = db::open(&app_data_dir).expect("failed to open database");
            app.manage(Db(Mutex::new(conn)));
            tray::setup(app.handle())?;
            desktop::setup(app.handle())?;
            reminders::setup(app.handle());

            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                match todoist::get_token() {
                    Ok(Some(_)) => {
                        if let Err(error) = todoist::sync::sync_now(&app_handle.state::<Db>()) {
                            eprintln!("Startup Todoist sync: {error}");
                        }
                    }
                    Ok(None) => {}
                    Err(_) => eprintln!("Startup Todoist sync: keychain unavailable"),
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                reminders::wake(window.app_handle());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let hide = window
                    .app_handle()
                    .try_state::<desktop::DesktopState>()
                    .and_then(|state| state.settings.lock().ok().map(|s| s.close_to_tray))
                    .unwrap_or(false);
                if hide && window.hide().is_ok() {
                    api.prevent_close();
                } else {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            reminders::notification_settings,
            reminders::save_notification_settings,
            reminders::test_notification,
            export::export_data,
            desktop::desktop_status,
            desktop::save_desktop_settings,
            desktop::set_autostart,
            tray::take_desktop_action,
            tray::list_startable_tasks,
            list_classes,
            create_class,
            update_class,
            archive_class,
            list_tasks_for_class,
            create_task,
            update_task,
            schedule_task,
            set_task_status,
            delete_task,
            get_today,
            get_week,
            get_study_capacity,
            set_study_capacity,
            get_active_session,
            start_timer,
            switch_timer,
            recover_timer,
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
