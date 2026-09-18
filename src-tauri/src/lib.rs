mod audio;
mod devices;
mod lifecycle;
mod settings;
mod tray;

use std::sync::Arc;

use tauri::{RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use devices::Devices;

// Cross-thread emits can deadlock WebView2 on Windows (tauri-apps/tauri#9453).
#[macro_export]
macro_rules! emit_on_main_thread {
    ($emitter:expr, $event:expr, $payload:expr) => {{
        let __emitter_clone = $emitter.clone();
        let __payload = $payload;
        let _ = $emitter.run_on_main_thread(move || {
            let _ = __emitter_clone.emit($event, __payload);
        });
    }};
}

pub fn run() {
    let devices = Arc::new(Devices::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            lifecycle::show_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![lifecycle::AUTOSTART_ARG]),
        ))
        .manage(devices.clone())
        .setup(move |app| {
            devices.attach(app.handle().clone());
            tray::set_visible(app.handle(), lifecycle::show_tray_enabled())?;
            audio::spawn(devices.clone());

            if !lifecycle::launched_by_autostart() {
                lifecycle::show_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            devices::commands::list_devices,
            devices::commands::set_target,
            devices::commands::set_locked,
            devices::commands::set_nickname,
            devices::commands::claim_device,
            devices::commands::forget_device,
            lifecycle::commands::get_autostart,
            lifecycle::commands::set_autostart,
            lifecycle::commands::get_show_tray,
            lifecycle::commands::set_show_tray,
            lifecycle::commands::hide_window,
            lifecycle::commands::quit,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build the app")
        .run(|_app, event| {
            if let RunEvent::ExitRequested { code: None, api, .. } = event {
                api.prevent_exit();
            }
        });
}
