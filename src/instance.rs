use std::sync::Arc;

use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, INFINITE, SetEvent, WaitForSingleObject,
};
use windows::core::PCWSTR;

use crate::state::State;
use crate::wide::wide;

const MUTEX_NAME: &str = r"Local\SimpleMicLock-instance";
const EVENT_NAME: &str = r"Local\SimpleMicLock-show";

pub struct Guard(HANDLE);

impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

pub enum Instance {
    First(Guard),
    AlreadyRunning,
}

pub fn claim() -> windows::core::Result<Instance> {
    let name = wide(MUTEX_NAME);

    let handle = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }?;

    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return Ok(Instance::AlreadyRunning);
    }

    Ok(Instance::First(Guard(handle)))
}

pub fn signal_existing() {
    let name = wide(EVENT_NAME);
    unsafe {
        if let Ok(event) = CreateEventW(None, false, false, PCWSTR(name.as_ptr())) {
            let _ = SetEvent(event);
            let _ = CloseHandle(event);
        }
    }
}

pub fn watch_for_relaunch(state: Arc<State>) {
    std::thread::Builder::new()
        .name("mic-lock-instance".into())
        .spawn(move || {
            let name = wide(EVENT_NAME);
            let Ok(event) = (unsafe { CreateEventW(None, false, false, PCWSTR(name.as_ptr())) })
            else {
                return;
            };

            while !state.quitting() {
                unsafe { WaitForSingleObject(event, INFINITE) };
                if state.quitting() {
                    break;
                }
                state.show_window();
            }

            unsafe {
                let _ = CloseHandle(event);
            }
        })
        .ok();
}
