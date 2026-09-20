use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

pub struct FocusState(AtomicBool);

pub fn setup(app: &AppHandle) {
    app.manage(FocusState(AtomicBool::new(false)));
}

/// Whether TaskTimer's own notifications (reminders, weekly review, exam
/// countdown) should be suppressed because the user turned on Focus Mode.
pub fn is_active(app: &AppHandle) -> bool {
    app.try_state::<FocusState>()
        .map(|s| s.0.load(Ordering::Relaxed))
        .unwrap_or(false)
}

#[tauri::command]
pub fn set_focus_mode(app: AppHandle, active: bool) {
    if let Some(state) = app.try_state::<FocusState>() {
        state.0.store(active, Ordering::Relaxed);
    }
}

#[tauri::command]
pub fn get_focus_mode(app: AppHandle) -> bool {
    is_active(&app)
}
