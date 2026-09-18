pub mod commands;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;

pub const AUTOSTART_ARG: &str = "--autostart";
pub const AUTOSTART_EVENT: &str = "autostart://changed";
pub const SHOW_TRAY_EVENT: &str = "tray://changed";

pub fn launched_by_autostart() -> bool {
    std::env::args().any(|arg| arg == AUTOSTART_ARG)
}

pub fn show_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else { return };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

pub fn hide_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

pub fn autostart_enabled(app: &AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

pub fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    let now = autostart_enabled(app);
    crate::emit_on_main_thread!(app, AUTOSTART_EVENT, now);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || crate::tray::refresh(&handle));
    result.map_err(|error| error.to_string())
}

pub fn show_tray_enabled() -> bool {
    crate::settings::get().show_tray
}

pub fn set_show_tray(app: &AppHandle, enabled: bool) -> Result<(), String> {
    crate::settings::update(|settings| settings.show_tray = enabled);
    crate::emit_on_main_thread!(app, SHOW_TRAY_EVENT, enabled);
    crate::tray::set_visible(app, enabled).map_err(|error| error.to_string())
}

pub fn quit(app: &AppHandle) {
    crate::devices::state(app).stop_audio();
    app.exit(0);
}
