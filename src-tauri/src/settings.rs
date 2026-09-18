use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    #[serde(default = "enabled")]
    pub show_tray: bool,
    #[serde(default)]
    pub devices: BTreeMap<String, DeviceSettings>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { show_tray: true, devices: BTreeMap::new() }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeviceSettings {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    pub target_percent: u32,
    pub locked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
}

static SETTINGS: LazyLock<Mutex<Settings>> = LazyLock::new(|| Mutex::new(load()));

fn enabled() -> bool {
    true
}

pub fn get() -> Settings {
    SETTINGS.lock().unwrap().clone()
}

pub fn update(change: impl FnOnce(&mut Settings)) {
    let mut settings = SETTINGS.lock().unwrap();
    change(&mut settings);
    save(&settings);
}

fn config_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("simple-mic-lock"))
}

fn load() -> Settings {
    let Some(dir) = config_dir() else {
        return Settings::default();
    };
    let path = dir.join("config.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Settings::default();
    };

    match serde_json::from_str::<Settings>(&text) {
        Ok(mut settings) => {
            for device in settings.devices.values_mut() {
                device.target_percent = device.target_percent.min(100);
            }
            settings
        }
        Err(_) => {
            let _ = std::fs::rename(&path, dir.join("config.invalid.json"));
            Settings::default()
        }
    }
}

fn save(settings: &Settings) {
    let Some(dir) = config_dir() else { return };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }

    let temp = dir.join("config.json.tmp");
    let Ok(text) = serde_json::to_string_pretty(settings) else { return };
    if std::fs::write(&temp, text).is_err() {
        let _ = std::fs::remove_file(&temp);
        return;
    }
    if std::fs::rename(&temp, dir.join("config.json")).is_err() {
        let _ = std::fs::remove_file(&temp);
    }
}
