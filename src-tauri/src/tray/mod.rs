use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Wry};

use crate::{devices, lifecycle};

const TRAY_ID: &str = "main";
const ICON_SIZE: u32 = 32;
const LOCKED_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-locked.rgba"));
const UNLOCKED_ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tray-unlocked.rgba"));

const OPEN_ID: &str = "open";
const AUTOSTART_ID: &str = "autostart";
const QUIT_ID: &str = "quit";
const LOCK_PREFIX: &str = "lock:";

pub fn set_visible(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    match (visible, app.tray_by_id(TRAY_ID).is_some()) {
        (true, false) => create(app),
        (false, true) => {
            app.remove_tray_by_id(TRAY_ID);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn create(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(false))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                lifecycle::show_window(tray.app_handle());
            }
        })
        .build(app)?;

    refresh(app);
    Ok(())
}

pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let summary = devices::state(app).summary();

    let _ = tray.set_icon(Some(icon(summary.locked > 0)));
    let _ = tray.set_tooltip(Some(format!("Simple Mic Lock\n{}", status(&summary))));
    if let Ok(menu) = menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn status(summary: &devices::Summary) -> String {
    match (summary.locked, summary.connected) {
        (_, 0) => "No microphone found".into(),
        (0, _) => "Nothing locked".into(),
        (locked, connected) => format!("{locked} of {connected} locked"),
    }
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    menu.append(&MenuItem::with_id(app, OPEN_ID, "Open Simple Mic Lock", true, None::<&str>)?)?;

    let entries = devices::state(app).tray_entries();
    if !entries.is_empty() {
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }
    for entry in entries {
        menu.append(&CheckMenuItem::with_id(
            app,
            format!("{LOCK_PREFIX}{}", entry.key),
            entry.label,
            entry.adjustable,
            entry.locked,
            None::<&str>,
        )?)?;
    }

    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&CheckMenuItem::with_id(
        app,
        AUTOSTART_ID,
        "Start with Windows",
        true,
        lifecycle::autostart_enabled(app),
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)?)?;
    Ok(menu)
}

fn on_menu(app: &AppHandle, id: &str) {
    match id {
        OPEN_ID => lifecycle::show_window(app),
        AUTOSTART_ID => {
            let _ = lifecycle::set_autostart(app, !lifecycle::autostart_enabled(app));
        }
        QUIT_ID => lifecycle::quit(app),
        _ => {
            if let Some(key) = id.strip_prefix(LOCK_PREFIX) {
                devices::state(app).toggle_locked(key);
            }
        }
    }
}

fn icon(locked: bool) -> Image<'static> {
    let rgba = if locked { LOCKED_ICON } else { UNLOCKED_ICON };
    Image::new(rgba, ICON_SIZE, ICON_SIZE)
}
