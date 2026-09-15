mod commands;
mod db;

use commands::classes::{archive_class, create_class, list_classes, update_class};
use db::Db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");
            let conn = db::open(&app_data_dir).expect("failed to open database");
            app.manage(Db(Mutex::new(conn)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_classes,
            create_class,
            update_class,
            archive_class,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
