# Simple Mic Lock

Keeps your microphone volume where you set it.

Some apps and drivers change your mic volume on their own. Simple Mic Lock changes it back right away.

## Features

- Lock each microphone at its own volume
- Give microphones nicknames
- Settings stay with the mic, even on a different USB port
- Runs quietly in the tray
- Can start with Windows

## Download

Download the installer from the [latest release](https://github.com/SalahaldinBilal/simple-mic-lock/releases/latest) and run it.

Windows may show "Windows protected your PC" because the app is not signed. Click **More info**, then **Run anyway**.

## How to use

1. Open Simple Mic Lock from the Start menu.
2. Drag a microphone's slider to the volume you want.
3. Click the lock icon next to it.

That's it. The volume now stays there until you unlock it.

- Click the pencil icon to rename a microphone.
- Closing the window keeps the app running in the tray. Use **Quit** to close it completely.
- The tray icon is green when at least one mic is locked and grey when none are.

## Two of the same mic

If you plug in two identical mics that have no serial number, the app can't tell them apart on its own. It will ask you which one is which.

## Settings file

Settings are saved in `%LOCALAPPDATA%\simple-mic-lock\config.json`.

## Requirements

Windows 10 or 11, 64-bit.

## Building from source

Install [Rust](https://rustup.rs), then run:

```
cargo build --release
```

The app will be at `target\release\simple-mic-lock.exe`.
