use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;

use crate::wide::wide;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "SimpleMicLock";

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn open(write: bool) -> Option<Key> {
    let path = wide(RUN_KEY);
    let mut key = HKEY::default();
    unsafe {
        let status = if write {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(path.as_ptr()),
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_READ | KEY_WRITE,
                None,
                &mut key,
                None,
            )
        } else {
            RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), None, KEY_READ, &mut key)
        };
        (status == ERROR_SUCCESS).then_some(Key(key))
    }
}

fn command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\" --hidden", exe.display()))
}

pub fn is_enabled() -> bool {
    let Some(key) = open(false) else { return false };
    let name = wide(VALUE_NAME);
    let mut size = 0u32;
    unsafe {
        let status = RegQueryValueExW(
            key.0,
            PCWSTR(name.as_ptr()),
            None,
            None,
            None,
            Some(&mut size),
        );
        status == ERROR_SUCCESS && size > 0
    }
}

pub fn set_enabled(enabled: bool) -> Result<(), String> {
    let key = open(true).ok_or("Could not open the current-user startup registry key.")?;
    let name = wide(VALUE_NAME);

    unsafe {
        let status = if enabled {
            let command = command().ok_or("Could not resolve this program's path.")?;
            let data = wide(&command);
            let bytes = std::slice::from_raw_parts(data.as_ptr().cast::<u8>(), data.len() * 2);
            RegSetValueExW(key.0, PCWSTR(name.as_ptr()), None, REG_SZ, Some(bytes))
        } else {
            RegDeleteValueW(key.0, PCWSTR(name.as_ptr()))
        };

        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("Registry update failed (code {}).", status.0))
        }
    }
}
