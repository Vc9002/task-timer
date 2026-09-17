mod commands;
mod db;
mod desktop;
mod export;
mod idle;
mod reminders;
mod timer;
mod todoist;
mod tray;
mod weekly_review;

use commands::analytics::{
    get_analytics_month, get_analytics_today, get_analytics_week, get_day_view,
    get_estimate_analytics, get_estimate_suggestion, get_task_history, get_weekly_review,
};
use commands::classes::{archive_class, create_class, list_classes, update_class};
use commands::exams::{create_exam, delete_exam, get_semester_dashboard, list_exams};
use commands::planner::{apply_plan, get_plan_proposal};
use commands::planning::{
    create_study_block, create_task_milestone, create_task_template, create_template_milestone,
    delete_study_block, delete_task_milestone, delete_task_template, delete_template_milestone,
    get_study_blocks_for_range, instantiate_task_template, list_study_blocks_for_task,
    list_task_milestones, list_task_templates, list_template_milestones, move_task_milestone,
    update_study_block, update_task_milestone,
};
use commands::pomodoro::{get_pomodoro_settings, notify_pomodoro_phase, save_pomodoro_settings};
use commands::recurrence::{
    create_recurring_template, get_calendar, list_recurring_templates,
    set_recurring_template_active, update_recurring_template,
};
use commands::tasks::{
    create_task, delete_task, duplicate_task, list_inbox, list_tasks_for_class, schedule_task,
    set_task_status, update_task,
};
use commands::today::get_today;
use commands::todoist::{
    complete_todoist_task, disconnect_todoist, get_todoist_outbox_status, get_todoist_status,
    list_todoist_project_mappings, map_todoist_project, set_todoist_token, sync_todoist_now,
};
use commands::week::{get_study_capacity, get_week, set_study_capacity};
use db::Db;
use std::sync::Mutex;
use tauri::Manager;
use timer::{
    cancel_timer, edit_session_duration, finish_timer, get_active_session, list_sessions_for_task,
    pause_timer, recover_timer, resume_timer, start_class_timer, start_timer, switch_timer,
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
            if let Err(error) = todoist::sync::recover_inflight_outbox(&app.state::<Db>()) {
                eprintln!("Todoist outbox recovery: {error}");
            }
            tray::setup(app.handle())?;
            desktop::setup(app.handle())?;
            reminders::setup(app.handle());
            idle::setup(app.handle());
            weekly_review::setup(app.handle());

            // Todoist is intentionally opt-in at runtime. Avoid reading the OS
            // keychain during startup; users can sync explicitly from Settings.

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
            idle::get_idle_threshold,
            idle::save_idle_threshold,
            weekly_review::get_weekly_review_settings,
            weekly_review::save_weekly_review_settings,
            export::export_data,
            export::import_backup,
            desktop::desktop_status,
            desktop::save_desktop_settings,
            desktop::set_autostart,
            tray::take_desktop_action,
            tray::list_startable_tasks,
            tray::set_tray_pomodoro_status,
            list_classes,
            create_class,
            update_class,
            archive_class,
            list_exams,
            create_exam,
            delete_exam,
            get_semester_dashboard,
            create_study_block,
            update_study_block,
            delete_study_block,
            list_study_blocks_for_task,
            get_study_blocks_for_range,
            list_task_milestones,
            create_task_milestone,
            update_task_milestone,
            delete_task_milestone,
            move_task_milestone,
            list_task_templates,
            create_task_template,
            delete_task_template,
            instantiate_task_template,
            list_template_milestones,
            create_template_milestone,
            delete_template_milestone,
            get_plan_proposal,
            apply_plan,
            get_pomodoro_settings,
            save_pomodoro_settings,
            notify_pomodoro_phase,
            list_tasks_for_class,
            list_inbox,
            create_task,
            update_task,
            duplicate_task,
            schedule_task,
            set_task_status,
            delete_task,
            get_today,
            get_week,
            get_study_capacity,
            set_study_capacity,
            list_recurring_templates,
            create_recurring_template,
            set_recurring_template_active,
            update_recurring_template,
            get_calendar,
            get_active_session,
            start_timer,
            start_class_timer,
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
            get_estimate_analytics,
            get_estimate_suggestion,
            get_weekly_review,
            get_todoist_status,
            get_todoist_outbox_status,
            set_todoist_token,
            disconnect_todoist,
            list_todoist_project_mappings,
            map_todoist_project,
            sync_todoist_now,
            complete_todoist_task,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
