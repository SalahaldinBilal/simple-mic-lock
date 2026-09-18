pub mod commands;
mod matching;
mod view;

use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};

use crate::audio;
use crate::settings::{self, DeviceSettings};
pub use view::Snapshot;

pub const CHANGED_EVENT: &str = "devices://changed";

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

pub struct Summary {
    pub connected: usize,
    pub locked: usize,
}

pub struct TrayEntry {
    pub key: String,
    pub label: String,
    pub locked: bool,
    pub adjustable: bool,
}

struct Inner {
    devices: Vec<Device>,
    unidentified: Vec<Unidentified>,
}

pub struct Devices {
    inner: Mutex<Inner>,
    audio_thread: AtomicU32,
    app: OnceLock<AppHandle>,
}

impl Devices {
    pub fn new() -> Self {
        let devices = settings::get()
            .devices
            .into_iter()
            .map(|(key, saved)| Device {
                key,
                name: saved.name,
                nickname: saved.nickname,
                target: saved.target_percent.min(100),
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
            audio_thread: AtomicU32::new(0),
            app: OnceLock::new(),
        }
    }

    pub fn attach(&self, app: AppHandle) {
        let _ = self.app.set(app);
    }

    pub fn set_audio_thread(&self, id: u32) {
        self.audio_thread.store(id, Ordering::Relaxed);
    }

    pub fn stop_audio(&self) {
        audio::stop(self.audio_thread.load(Ordering::Relaxed));
    }

    pub fn snapshot(&self) -> Snapshot {
        let inner = self.inner.lock().unwrap();
        Snapshot::new(&inner.devices, &inner.unidentified)
    }

    pub fn summary(&self) -> Summary {
        let inner = self.inner.lock().unwrap();
        let present = inner.devices.iter().filter(|device| device.present());
        Summary {
            locked: present.clone().filter(|device| device.locked).count(),
            connected: present.count() + inner.unidentified.len(),
        }
    }

    pub fn tray_entries(&self) -> Vec<TrayEntry> {
        let inner = self.inner.lock().unwrap();
        inner
            .devices
            .iter()
            .filter(|device| device.present())
            .map(|device| TrayEntry {
                key: device.key.clone(),
                label: format!("{} · {}%", device.display_name(), device.target),
                locked: device.locked,
                adjustable: device.adjustable,
            })
            .collect()
    }

    pub fn enforcement(&self, endpoint: &str) -> Option<(f32, bool)> {
        let inner = self.inner.lock().unwrap();
        inner
            .devices
            .iter()
            .find(|device| device.endpoint.as_deref() == Some(endpoint))
            .map(|device| (device.target as f32 / 100.0, device.locked))
    }

    pub fn set_target(&self, key: &str, percent: u32) {
        let percent = percent.min(100);
        if self.edit(key, |device| std::mem::replace(&mut device.target, percent) != percent) {
            self.settings_changed();
        }
    }

    pub fn set_locked(&self, key: &str, locked: bool) {
        if self.edit(key, |device| std::mem::replace(&mut device.locked, locked) != locked) {
            self.settings_changed();
        }
    }

    pub fn toggle_locked(&self, key: &str) {
        if self.edit(key, |device| {
            device.locked = !device.locked;
            true
        }) {
            self.settings_changed();
        }
    }

    pub fn set_nickname(&self, key: &str, nickname: Option<String>) {
        let nickname = nickname
            .map(|nickname| nickname.trim().to_owned())
            .filter(|nickname| !nickname.is_empty());
        let changed = self.edit(key, |device| {
            let changed = device.nickname != nickname;
            device.nickname = nickname;
            changed
        });
        if changed {
            self.settings_changed();
        }
    }

    pub fn forget(&self, key: &str) {
        let removed = {
            let mut inner = self.inner.lock().unwrap();
            let before = inner.devices.len();
            inner.devices.retain(|device| device.key != key || device.present());
            inner.devices.len() != before
        };
        if removed {
            self.settings_changed();
        }
    }

    pub fn claim(&self, endpoint: &str, choice: Option<String>) {
        {
            let mut inner = self.inner.lock().unwrap();
            let Inner { devices, unidentified } = &mut *inner;
            matching::claim(devices, unidentified, endpoint, choice.as_deref());
            sort(devices);
        }
        self.settings_changed();
    }

    pub fn set_level(&self, endpoint: &str, level: Option<f32>) {
        let changed = {
            let mut inner = self.inner.lock().unwrap();
            let Inner { devices, unidentified } = &mut *inner;
            if let Some(device) =
                devices.iter_mut().find(|device| device.endpoint.as_deref() == Some(endpoint))
            {
                std::mem::replace(&mut device.level, level) != level
            } else if let Some(unit) =
                unidentified.iter_mut().find(|unit| unit.discovered.endpoint == endpoint)
            {
                std::mem::replace(&mut unit.discovered.level, level) != level
            } else {
                false
            }
        };
        if changed {
            self.emit();
        }
    }

    pub fn sync(&self, discovered: Vec<Discovered>) {
        let newly_unidentified = {
            let mut inner = self.inner.lock().unwrap();
            let Inner { devices, unidentified } = &mut *inner;
            let waiting: HashSet<String> =
                unidentified.iter().map(|unit| unit.discovered.endpoint.clone()).collect();
            *unidentified = matching::apply(devices, discovered);
            sort(devices);
            unidentified.iter().any(|unit| !waiting.contains(&unit.discovered.endpoint))
        };

        self.persist();
        self.emit();
        self.refresh_tray();

        if newly_unidentified {
            if let Some(app) = self.app.get() {
                let app = app.clone();
                let _ = app.clone().run_on_main_thread(move || crate::lifecycle::show_window(&app));
            }
        }
    }

    fn edit(&self, key: &str, change: impl FnOnce(&mut Device) -> bool) -> bool {
        let mut inner = self.inner.lock().unwrap();
        match inner.devices.iter_mut().find(|device| device.key == key) {
            Some(device) => change(device),
            None => false,
        }
    }

    fn settings_changed(&self) {
        self.persist();
        audio::sync(self.audio_thread.load(Ordering::Relaxed));
        self.emit();
        self.refresh_tray();
    }

    fn emit(&self) {
        let Some(app) = self.app.get() else { return };
        crate::emit_on_main_thread!(app, CHANGED_EVENT, self.snapshot());
    }

    fn refresh_tray(&self) {
        let Some(app) = self.app.get() else { return };
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || crate::tray::refresh(&handle));
    }

    fn persist(&self) {
        let saved: BTreeMap<String, DeviceSettings> = {
            let inner = self.inner.lock().unwrap();
            inner
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
                .collect()
        };
        settings::update(|settings| settings.devices = saved);
    }
}

pub fn state(app: &AppHandle) -> tauri::State<'_, std::sync::Arc<Devices>> {
    app.state::<std::sync::Arc<Devices>>()
}

fn sort(devices: &mut [Device]) {
    devices.sort_by(|a, b| {
        b.present()
            .cmp(&a.present())
            .then(b.is_default.cmp(&a.is_default))
            .then_with(|| a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()))
    });
}
