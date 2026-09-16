// Dev builds keep the console so Ctrl+C in the terminal reaches the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod autorun;
mod icon;
mod identity;
mod instance;
mod matching;
mod settings;
mod state;
mod theme;
mod tray;
mod ui;
mod wide;

use std::sync::{Arc, OnceLock};

use instance::Instance;
use state::State;
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows_core::BOOL;

pub const APP_NAME: &str = env!("APP_NAME");

static CONSOLE_QUIT_TARGET: OnceLock<Arc<State>> = OnceLock::new();

fn main() {
    let _guard = match instance::claim() {
        Ok(Instance::First(guard)) => Some(guard),
        Ok(Instance::AlreadyRunning) => {
            instance::signal_existing();
            return;
        }
        Err(_) => None,
    };

    let start_hidden = std::env::args().any(|arg| arg.eq_ignore_ascii_case("--hidden"));

    let state = Arc::new(State::new(settings::load()));
    quit_on_console_signal(state.clone());
    instance::watch_for_relaunch(state.clone());
    let audio = audio::spawn(state.clone());
    let tray = tray::spawn(state.clone());

    ui::run(state.clone(), start_hidden);

    state.request_quit();
    let _ = tray.join();
    let _ = audio.join();
}

fn quit_on_console_signal(state: Arc<State>) {
    unsafe extern "system" fn handler(_signal: u32) -> BOOL {
        if let Some(state) = CONSOLE_QUIT_TARGET.get() {
            state.request_quit();
        }
        BOOL::from(true)
    }

    let _ = CONSOLE_QUIT_TARGET.set(state);
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }
}
