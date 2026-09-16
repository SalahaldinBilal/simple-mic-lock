use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP, WM_QUIT};

use crate::matching;
use crate::settings::{DeviceSettings, Settings};

pub const MSG_SYNC: u32 = WM_APP + 1;
pub const MSG_VOLUME: u32 = WM_APP + 2;
pub const MSG_DEVICE: u32 = WM_APP + 3;

#[derive(Clone, Debug)]
pub struct Device {
    pub key: String,
    pub name: String,
    pub nickname: Option<String>,
    pub target: u32,
    pub locked: bool,
    pub model: Option<String>,
    pub port: Option<String>,
    pub endpoint: Option<String>,
    pub adjustable: bool,
    pub is_default: bool,
    pub level: Option<f32>,
}

impl Device {
    pub fn present(&self) -> bool {
        self.endpoint.is_some()
    }

    pub fn display_name(&self) -> &str {
        self.nickname.as_deref().unwrap_or(&self.name)
    }

    pub fn target_scalar(&self) -> f32 {
        self.target as f32 / 100.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hardware {
    Endpoint,
    Serial { model: String, serial: String },
    Anonymous { model: String, port: String },
}

#[derive(Clone, Debug)]
pub struct Discovered {
    pub endpoint: String,
    pub hardware: Hardware,
    pub name: Option<String>,
    pub adjustable: bool,
    pub is_default: bool,
    pub level: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct Unidentified {
    pub discovered: Discovered,
    pub prompt: Prompt,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Prompt {
    Choose(Vec<Candidate>),
    KeepOnlyOne(Option<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub key: String,
    pub name: String,
}

struct Inner {
    devices: Vec<Device>,
    unidentified: Vec<Unidentified>,
}

pub struct State {
    inner: Mutex<Inner>,
    autorun: AtomicBool,
    audio_thread: AtomicU32,
    tray_thread: AtomicU32,
    ui: Mutex<Option<egui::Context>>,
    quitting: AtomicBool,
}

impl State {
    pub fn new(settings: Settings) -> Self {
        let devices = settings
            .devices
            .into_iter()
            .map(|(key, saved)| Device {
                key,
                name: saved.name,
                nickname: saved.nickname,
                target: saved.target_percent.clamp(0, 100),
                locked: saved.locked,
                model: saved.model,
                port: saved.port,
                endpoint: None,
                adjustable: false,
                is_default: false,
                level: None,
            })
            .collect();

        Self {
            inner: Mutex::new(Inner { devices, unidentified: Vec::new() }),
            autorun: AtomicBool::new(crate::autorun::is_enabled()),
            audio_thread: AtomicU32::new(0),
            tray_thread: AtomicU32::new(0),
            ui: Mutex::new(None),
            quitting: AtomicBool::new(false),
        }
    }

    pub fn devices(&self) -> Vec<Device> {
        self.inner.lock().unwrap().devices.clone()
    }

    pub fn unidentified(&self) -> Vec<Unidentified> {
        self.inner.lock().unwrap().unidentified.clone()
    }

    pub fn enforcement(&self, endpoint: &str) -> Option<(f32, bool)> {
        let inner = self.inner.lock().unwrap();
        inner
            .devices
            .iter()
            .find(|device| device.endpoint.as_deref() == Some(endpoint))
            .map(|device| (device.target_scalar(), device.locked))
    }

    pub fn set_target(&self, key: &str, percent: u32) {
        let percent = percent.clamp(0, 100);
        if self.edit(key, |device| std::mem::replace(&mut device.target, percent) != percent) {
            self.broadcast();
        }
    }

    pub fn set_locked(&self, key: &str, locked: bool) {
        if self.edit(key, |device| std::mem::replace(&mut device.locked, locked) != locked) {
            self.broadcast();
        }
    }

    pub fn set_nickname(&self, key: &str, nickname: Option<String>) {
        let changed = self.edit(key, |device| {
            let changed = device.nickname != nickname;
            device.nickname = nickname;
            changed
        });
        if changed {
            self.broadcast();
        }
    }

    pub fn set_level(&self, endpoint: &str, level: Option<f32>) {
        let changed = {
            let mut inner = self.inner.lock().unwrap();
            let Inner { devices, unidentified } = &mut *inner;
            if let Some(device) = devices.iter_mut().find(|device| device.endpoint.as_deref() == Some(endpoint)) {
                std::mem::replace(&mut device.level, level) != level
            } else if let Some(unit) = unidentified.iter_mut().find(|unit| unit.discovered.endpoint == endpoint) {
                std::mem::replace(&mut unit.discovered.level, level) != level
            } else {
                false
            }
        };
        if changed {
            self.notify_ui();
        }
    }

    pub fn sync_devices(&self, discovered: Vec<Discovered>) {
        {
            let mut inner = self.inner.lock().unwrap();
            let Inner { devices, unidentified } = &mut *inner;
            *unidentified = matching::apply(devices, discovered);
            sort(devices);
        }
        self.persist();
        self.notify_tray();
        self.notify_ui();
    }

    pub fn claim(&self, endpoint: &str, choice: Option<String>) {
        {
            let mut inner = self.inner.lock().unwrap();
            let Inner { devices, unidentified } = &mut *inner;
            matching::claim(devices, unidentified, endpoint, choice.as_deref());
            sort(devices);
        }
        self.broadcast();
    }

    pub fn locked_count(&self) -> (usize, usize) {
        let inner = self.inner.lock().unwrap();
        let present = inner.devices.iter().filter(|device| device.present());
        let locked = present.clone().filter(|device| device.locked).count();
        (locked, present.count() + inner.unidentified.len())
    }

    pub fn autorun(&self) -> bool {
        self.autorun.load(Ordering::Relaxed)
    }

    pub fn set_autorun(&self, enabled: bool) -> Result<(), String> {
        crate::autorun::set_enabled(enabled)?;
        self.autorun.store(enabled, Ordering::Relaxed);
        self.notify_tray();
        self.notify_ui();
        Ok(())
    }

    pub fn attach_ui(&self, ctx: egui::Context) {
        *self.ui.lock().unwrap() = Some(ctx);
    }

    pub fn set_audio_thread(&self, id: u32) {
        self.audio_thread.store(id, Ordering::Relaxed);
    }

    pub fn set_tray_thread(&self, id: u32) {
        self.tray_thread.store(id, Ordering::Relaxed);
    }

    pub fn quitting(&self) -> bool {
        self.quitting.load(Ordering::Relaxed)
    }

    pub fn show_window(&self) {
        if let Some(ctx) = self.ui.lock().unwrap().as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.request_repaint();
        }
    }

    pub fn hide_window(&self) {
        if let Some(ctx) = self.ui.lock().unwrap().as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    pub fn request_quit(&self) {
        self.quitting.store(true, Ordering::Relaxed);
        post(self.audio_thread.load(Ordering::Relaxed), WM_QUIT, 0, 0);
        post(self.tray_thread.load(Ordering::Relaxed), WM_QUIT, 0, 0);
        if let Some(ctx) = self.ui.lock().unwrap().as_ref() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            ctx.request_repaint();
        }
    }

    pub fn notify_audio(&self) {
        post(self.audio_thread.load(Ordering::Relaxed), MSG_SYNC, 0, 0);
    }

    pub fn notify_tray(&self) {
        post(self.tray_thread.load(Ordering::Relaxed), MSG_SYNC, 0, 0);
    }

    pub fn notify_ui(&self) {
        if let Some(ctx) = self.ui.lock().unwrap().as_ref() {
            ctx.request_repaint();
        }
    }

    fn edit(&self, key: &str, change: impl FnOnce(&mut Device) -> bool) -> bool {
        let mut inner = self.inner.lock().unwrap();
        match inner.devices.iter_mut().find(|device| device.key == key) {
            Some(device) => change(device),
            None => false,
        }
    }

    fn broadcast(&self) {
        self.persist();
        self.notify_audio();
        self.notify_tray();
        self.notify_ui();
    }

    fn persist(&self) {
        let inner = self.inner.lock().unwrap();
        let settings = Settings {
            devices: inner
                .devices
                .iter()
                .map(|device| {
                    (
                        device.key.clone(),
                        DeviceSettings {
                            name: device.name.clone(),
                            nickname: device.nickname.clone(),
                            target_percent: device.target,
                            locked: device.locked,
                            model: device.model.clone(),
                            port: device.port.clone(),
                        },
                    )
                })
                .collect(),
        };
        drop(inner);
        crate::settings::save(&settings);
    }
}

fn sort(devices: &mut [Device]) {
    devices.sort_by(|a, b| {
        b.present()
            .cmp(&a.present())
            .then(b.is_default.cmp(&a.is_default))
            .then_with(|| a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()))
    });
}

pub fn post(thread: u32, message: u32, wparam: usize, lparam: isize) {
    if thread == 0 {
        return;
    }
    unsafe {
        let _ = PostThreadMessageW(thread, message, WPARAM(wparam), LPARAM(lparam));
    }
}
