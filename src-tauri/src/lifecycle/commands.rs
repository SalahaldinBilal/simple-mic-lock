use tauri::AppHandle;

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    super::autostart_enabled(&app)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    super::set_autostart(&app, enabled)
}

#[tauri::command]
pub fn get_show_tray() -> bool {
    super::show_tray_enabled()
}

#[tauri::command]
pub fn set_show_tray(app: AppHandle, enabled: bool) -> Result<(), String> {
    super::set_show_tray(&app, enabled)
}

#[tauri::command]
pub fn hide_window(app: AppHandle) {
    super::hide_window(&app);
}

#[tauri::command]
pub fn quit(app: AppHandle) {
    super::quit(&app);
}
