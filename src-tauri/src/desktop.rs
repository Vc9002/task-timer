use crate::{db::Db, tray};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub close_to_tray: bool,
    pub start_hidden: bool,
    pub shortcuts: Vec<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: cfg!(any(target_os = "macos", target_os = "windows")),
            start_hidden: false,
            shortcuts: vec![
                "CommandOrControl+Shift+Y".into(),
                "CommandOrControl+Shift+Space".into(),
                "CommandOrControl+Shift+F".into(),
                "CommandOrControl+Shift+A".into(),
            ],
        }
    }
}
pub struct DesktopState {
    pub settings: Mutex<Settings>,
    update: Mutex<()>,
    warning: Mutex<Option<String>>,
}
#[derive(Serialize)]
pub struct Status {
    settings: Settings,
    autostart: bool,
    shortcut_warning: Option<String>,
}

fn parse_shortcuts(settings: &Settings) -> Result<Vec<(Shortcut, usize)>, String> {
    let mut result = Vec::new();
    for (i, text) in settings.shortcuts.iter().enumerate() {
        if text.trim().is_empty() {
            continue;
        }
        let shortcut: Shortcut = text
            .parse()
            .map_err(|_| "Invalid shortcut. Use CommandOrControl+Shift+Y, for example.")?;
        if shortcut.mods.is_empty() {
            return Err("Global shortcuts must include a modifier key.".into());
        }
        if result.iter().any(|(old, _)| old == &shortcut) {
            return Err("Choose a different shortcut for each action.".into());
        }
        result.push((shortcut, i));
    }
    Ok(result)
}
fn register(app: &AppHandle, shortcut: Shortcut, action: usize) -> Result<(), String> {
    app.global_shortcut().on_shortcut(shortcut, move |app, _, event| {
        if event.state == ShortcutState::Pressed {
            match action { 0 => tray::open_action(app,"start-task"), 1 => tray::timer_action(app,"pause"), 2 => tray::timer_action(app,"finish"), _ => tray::show_quick_capture(app) }
        }
    }).map_err(|_| "That shortcut couldn't be registered. It may be used by another application; choose another combination.".into())
}
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let settings = {
        let db = app.state::<Db>();
        let conn = db.0.lock().map_err(|_| "Settings unavailable")?;
        let json: Option<String> = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key='desktop_v02'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let mut settings = json
            .map(|s| serde_json::from_str::<Settings>(&s))
            .transpose()?
            .unwrap_or_default();
        // Cmd/Ctrl+Shift+T is Chrome's reopen-closed-tab shortcut. Migrate
        // only the shipped default; preserve any user-selected shortcut.
        let mut dirty = false;
        if settings.shortcuts.first().map(String::as_str) == Some("CommandOrControl+Shift+T") {
            settings.shortcuts[0] = "CommandOrControl+Shift+Y".into();
            dirty = true;
        }
        // Migrate settings saved before the quick-add shortcut slot existed.
        if settings.shortcuts.len() < 4 {
            settings.shortcuts.push("CommandOrControl+Shift+A".into());
            dirty = true;
        }
        if dirty {
            conn.execute(
                "INSERT INTO app_settings(key,value) VALUES('desktop_v02',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [serde_json::to_string(&settings)?],
            )?;
        }
        settings
    };
    let mut warning = None;
    match parse_shortcuts(&settings) {
        Ok(shortcuts) => {
            for (shortcut, action) in shortcuts {
                if let Err(error) = register(app, shortcut, action) {
                    warning = Some(error);
                }
            }
        }
        Err(error) => warning = Some(error),
    }
    let hidden = settings.start_hidden && std::env::args().any(|arg| arg == "--autostart");
    app.manage(DesktopState {
        settings: Mutex::new(settings),
        update: Mutex::new(()),
        warning: Mutex::new(warning),
    });
    if hidden {
        if let Some(window) = app.get_webview_window("main") {
            window.hide()?;
        }
    }
    Ok(())
}
#[tauri::command]
pub fn desktop_status(app: AppHandle) -> Result<Status, String> {
    let state = app.state::<DesktopState>();
    let result = Status {
        settings: state
            .settings
            .lock()
            .map_err(|_| "Settings unavailable")?
            .clone(),
        autostart: app
            .autolaunch()
            .is_enabled()
            .map_err(|_| "Couldn't check login settings")?,
        shortcut_warning: state
            .warning
            .lock()
            .map_err(|_| "Settings unavailable")?
            .clone(),
    };
    Ok(result)
}
#[tauri::command]
pub async fn save_desktop_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<DesktopState>();
        let _update = state.update.lock().map_err(|_| "Settings unavailable")?;
        let next = parse_shortcuts(&settings)?;
        let previous = state.settings.lock().map_err(|_| "Settings unavailable")?.clone();
        let old = parse_shortcuts(&previous)?;
        app.global_shortcut().unregister_all().map_err(|_| "Couldn't update shortcuts")?;
        let apply = (|| {
            for (shortcut, action) in &next { register(&app, *shortcut, *action)?; }
            let db = app.state::<Db>();
            let conn = db.0.lock().map_err(|_| "Couldn't save settings")?;
            conn.execute("INSERT INTO app_settings(key,value) VALUES('desktop_v02',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(&settings).map_err(|_| "Couldn't save settings")?]).map_err(|_| "Couldn't save settings")?;
            Ok::<_,String>(())
        })();
        if let Err(error) = apply {
            let _ = app.global_shortcut().unregister_all();
            let mut restore_failed = false;
            for (shortcut, action) in old { restore_failed |= register(&app,shortcut,action).is_err(); }
            if restore_failed { *state.warning.lock().map_err(|_| "Settings unavailable")? = Some("Some previous shortcuts couldn't be restored. Choose new shortcuts.".into()); }
            return Err(error);
        }
        *state.settings.lock().map_err(|_| "Settings unavailable")? = settings;
        *state.warning.lock().map_err(|_| "Settings unavailable")? = None;
        Ok(())
    }).await.map_err(|_| "Couldn't update desktop settings")?
}
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|_| {
        "Couldn't change login settings. Check your operating system's login-item permissions."
            .into()
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_disabled_shortcuts_are_valid() {
        assert_eq!(parse_shortcuts(&Settings::default()).unwrap().len(), 4);
        assert!(parse_shortcuts(&Settings {
            shortcuts: Default::default(),
            ..Settings::default()
        })
        .unwrap()
        .is_empty());
    }
    #[test]
    fn shortcuts_reject_duplicates_and_unmodified_keys() {
        let mut settings = Settings::default();
        settings.shortcuts[1] = settings.shortcuts[0].clone();
        assert!(parse_shortcuts(&settings).is_err());
        settings.shortcuts[1] = "A".into();
        assert!(parse_shortcuts(&settings).is_err());
    }
}
