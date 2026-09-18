use std::sync::Arc;

use tauri::State;

use super::{Devices, Snapshot};

#[tauri::command]
pub fn list_devices(devices: State<'_, Arc<Devices>>) -> Snapshot {
    devices.snapshot()
}

#[tauri::command]
pub fn set_target(devices: State<'_, Arc<Devices>>, key: String, percent: u32) {
    devices.set_target(&key, percent);
}

#[tauri::command]
pub fn set_locked(devices: State<'_, Arc<Devices>>, key: String, locked: bool) {
    devices.set_locked(&key, locked);
}

#[tauri::command]
pub fn set_nickname(devices: State<'_, Arc<Devices>>, key: String, nickname: Option<String>) {
    devices.set_nickname(&key, nickname);
}

#[tauri::command]
pub fn claim_device(devices: State<'_, Arc<Devices>>, endpoint: String, key: Option<String>) {
    devices.claim(&endpoint, key);
}

#[tauri::command]
pub fn forget_device(devices: State<'_, Arc<Devices>>, key: String) {
    devices.forget(&key);
}
