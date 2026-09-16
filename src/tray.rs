use std::sync::Arc;
use std::thread::JoinHandle;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, TranslateMessage};

use crate::APP_NAME;
use crate::icon;
use crate::state::{Device, MSG_SYNC, State};

const ID_OPEN: &str = "open";
const ID_AUTORUN: &str = "autorun";
const ID_QUIT: &str = "quit";
const DEVICE_PREFIX: &str = "mic:";

pub fn spawn(state: Arc<State>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("mic-lock-tray".into())
        .spawn(move || run(state))
        .expect("failed to start the tray thread")
}

fn run(state: Arc<State>) {
    let mut menu = DeviceMenu::build(&state);

    let Ok(tray) = TrayIconBuilder::new()
        .with_menu(Box::new(menu.take_menu()))
        .with_tooltip(tooltip(&state))
        .with_icon(glyph(state.locked_count().0 > 0))
        .with_menu_on_left_click(false)
        .build()
    else {
        return;
    };

    let mut shown_as_locked = state.locked_count().0 > 0;

    // Only notifiable once the message window exists, so catch up on anything that changed before.
    state.set_tray_thread(unsafe { GetCurrentThreadId() });
    sync(&state, &tray, &mut menu, &mut shown_as_locked);
    let menu_events = MenuEvent::receiver();
    let tray_events = TrayIconEvent::receiver();
    let mut msg = MSG::default();

    loop {
        if unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 <= 0 {
            break;
        }

        if msg.hwnd.is_invalid() && msg.message == MSG_SYNC {
            sync(&state, &tray, &mut menu, &mut shown_as_locked);
        } else {
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        while let Ok(event) = menu_events.try_recv() {
            let id = event.id.as_ref();
            match id {
                ID_OPEN => state.show_window(),
                ID_QUIT => state.request_quit(),
                ID_AUTORUN => {
                    let wanted = menu.autorun.is_checked();
                    if state.set_autorun(wanted).is_err() {
                        menu.autorun.set_checked(!wanted);
                    }
                }
                _ => {
                    if let Some(key) = id.strip_prefix(DEVICE_PREFIX) {
                        if let Some(item) = menu.device_item(key) {
                            state.set_locked(key, item.is_checked());
                        }
                    }
                }
            }
        }

        while let Ok(event) = tray_events.try_recv() {
            let opens = match event {
                TrayIconEvent::Click { button, button_state, .. } => {
                    button == MouseButton::Left && button_state == MouseButtonState::Up
                }
                TrayIconEvent::DoubleClick { .. } => true,
                _ => false,
            };
            if opens {
                state.show_window();
            }
        }

        if state.quitting() {
            break;
        }
    }

    drop::<TrayIcon>(tray);
}

fn sync(state: &State, tray: &TrayIcon, menu: &mut DeviceMenu, shown_as_locked: &mut bool) {
    if menu.is_stale(state) {
        *menu = DeviceMenu::build(state);
        tray.set_menu(Some(Box::new(menu.take_menu())));
    } else {
        menu.refresh(state);
    }

    let _ = tray.set_tooltip(Some(tooltip(state)));
    let locked = state.locked_count().0 > 0;
    if *shown_as_locked != locked {
        *shown_as_locked = locked;
        let _ = tray.set_icon(Some(glyph(locked)));
    }
}

struct DeviceMenu {
    menu: Option<Menu>,
    devices: Vec<(String, CheckMenuItem)>,
    autorun: CheckMenuItem,
}

impl DeviceMenu {
    fn build(state: &State) -> Self {
        let menu = Menu::new();
        let _ = menu.append(&MenuItem::with_id(ID_OPEN, "Open Simple Mic Lock", true, None));
        let _ = menu.append(&PredefinedMenuItem::separator());

        let present = present_devices(state);
        let mut devices = Vec::new();

        if present.is_empty() {
            let _ = menu.append(&MenuItem::with_id("none", "No microphone found", false, None));
        }

        for device in present {
            let item = CheckMenuItem::with_id(
                format!("{DEVICE_PREFIX}{}", device.key),
                device_label(&device),
                device.adjustable,
                device.locked,
                None,
            );
            let _ = menu.append(&item);
            devices.push((device.key, item));
        }

        let _ = menu.append(&PredefinedMenuItem::separator());
        let autorun =
            CheckMenuItem::with_id(ID_AUTORUN, "Start with Windows", true, state.autorun(), None);
        let _ = menu.append(&autorun);
        let _ = menu.append(&PredefinedMenuItem::separator());
        let _ = menu.append(&MenuItem::with_id(ID_QUIT, "Quit", true, None));

        Self { menu: Some(menu), devices, autorun }
    }

    fn take_menu(&mut self) -> Menu {
        self.menu.take().expect("menu handed over exactly once")
    }

    fn is_stale(&self, state: &State) -> bool {
        let present = present_devices(state);
        present.len() != self.devices.len()
            || present.iter().zip(&self.devices).any(|(device, (known, _))| &device.key != known)
    }

    fn refresh(&self, state: &State) {
        self.autorun.set_checked(state.autorun());
        for device in state.devices() {
            if let Some(item) = self.device_item(&device.key) {
                item.set_checked(device.locked);
                item.set_text(device_label(&device));
            }
        }
    }

    fn device_item(&self, key: &str) -> Option<&CheckMenuItem> {
        self.devices
            .iter()
            .find(|(known, _)| known == key)
            .map(|(_, item)| item)
    }
}

fn present_devices(state: &State) -> Vec<Device> {
    state.devices().into_iter().filter(Device::present).collect()
}

fn device_label(device: &Device) -> String {
    format!("{} · {}%", device.display_name(), device.target)
}

fn tooltip(state: &State) -> String {
    let status = match state.locked_count() {
        (_, 0) => "No microphone found".to_string(),
        (0, _) => "Nothing locked".to_string(),
        (locked, total) if locked == total => format!("{locked} locked"),
        (locked, total) => format!("{locked} of {total} locked"),
    };
    format!("{APP_NAME}\n{status}")
}

fn glyph(locked: bool) -> Icon {
    let art = icon::tray(locked);
    Icon::from_rgba(art.pixels, art.width, art.height).expect("tray icon has a valid size")
}
