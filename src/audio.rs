use std::sync::Arc;
use std::thread::JoinHandle;

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IMMDevice,
    IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
    eCapture, eConsole,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
    CoUninitialize, STGM_READ,
};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    GetMessageW, KillTimer, MSG, PM_NOREMOVE, PeekMessageW, SetTimer, WM_QUIT, WM_TIMER, WM_USER,
};
use windows_core::{GUID, PCWSTR, Result, implement};

use crate::identity;
use crate::state::{Discovered, MSG_DEVICE, MSG_SYNC, MSG_VOLUME, State, post};

const EVENT_CONTEXT: GUID = GUID::from_u128(0x7c3e9a52_4f1d_4b8e_a6d0_2b9c5e81f437);
const TOLERANCE: f32 = 0.005;
const REFRESH_DELAY_MS: u32 = 150;
const POLL_INTERVAL_MS: u32 = 1000;

pub fn spawn(state: Arc<State>) -> JoinHandle<()> {
    std::thread::Builder::new()
        .name("mic-lock-audio".into())
        .spawn(move || run(state))
        .expect("failed to start the audio thread")
}

fn run(state: Arc<State>) {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let mut msg = MSG::default();
        let _ = PeekMessageW(&mut msg, None, WM_USER, WM_USER, PM_NOREMOVE);
        state.set_audio_thread(GetCurrentThreadId());

        let mut audio = Audio::new(state);
        audio.refresh();

        let poll_timer = SetTimer(None, 0, POLL_INTERVAL_MS, None);
        let mut refresh_timer = 0usize;

        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            match msg.message {
                MSG_SYNC => audio.enforce_all(),
                MSG_VOLUME => {
                    let observed = f32::from_bits(msg.lParam.0 as u32);
                    audio.on_volume_changed(msg.wParam.0, observed);
                }
                MSG_DEVICE => {
                    if refresh_timer != 0 {
                        let _ = KillTimer(None, refresh_timer);
                    }
                    refresh_timer = SetTimer(None, refresh_timer, REFRESH_DELAY_MS, None);
                }
                WM_TIMER if msg.wParam.0 == poll_timer => audio.poll(),
                WM_TIMER if msg.wParam.0 == refresh_timer => {
                    let _ = KillTimer(None, refresh_timer);
                    refresh_timer = 0;
                    audio.refresh();
                }
                WM_QUIT => break,
                _ => {}
            }
        }

        let _ = KillTimer(None, poll_timer);
        drop(audio);
        CoUninitialize();
    }
}

struct Binding {
    token: usize,
    endpoint: String,
    volume: IAudioEndpointVolume,
    callback: IAudioEndpointVolumeCallback,
    adjustable: bool,
}

struct Audio {
    state: Arc<State>,
    thread: u32,
    next_token: usize,
    enumerator: Option<IMMDeviceEnumerator>,
    notifications: Option<IMMNotificationClient>,
    bindings: Vec<Binding>,
}

impl Audio {
    fn new(state: Arc<State>) -> Self {
        Self {
            state,
            thread: unsafe { GetCurrentThreadId() },
            next_token: 1,
            enumerator: None,
            notifications: None,
            bindings: Vec::new(),
        }
    }

    fn enumerator(&mut self) -> Option<IMMDeviceEnumerator> {
        if self.enumerator.is_none() {
            let created: Result<IMMDeviceEnumerator> =
                unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) };
            let Ok(enumerator) = created else { return None };

            let client: IMMNotificationClient = DeviceNotifications { thread: self.thread }.into();
            unsafe {
                if enumerator.RegisterEndpointNotificationCallback(&client).is_ok() {
                    self.notifications = Some(client);
                }
            }
            self.enumerator = Some(enumerator);
        }
        self.enumerator.clone()
    }

    fn refresh(&mut self) {
        let Some(enumerator) = self.enumerator() else { return };
        let Some(devices) = active_capture_devices(&enumerator) else { return };

        self.release_all();

        let default_endpoint = unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, eConsole) }
            .ok()
            .and_then(|device| endpoint_id(&device));

        let discovered = devices
            .iter()
            .filter_map(|device| {
                let endpoint = endpoint_id(device)?;
                self.bind(device, endpoint, default_endpoint.as_deref())
            })
            .collect();

        self.state.sync_devices(discovered);
        self.enforce_all();
    }

    fn bind(
        &mut self,
        device: &IMMDevice,
        endpoint: String,
        default_endpoint: Option<&str>,
    ) -> Option<Discovered> {
        unsafe {
            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;

            // Reading the level is the only reliable test for a usable volume node.
            let level = volume.GetMasterVolumeLevelScalar().ok();
            let adjustable = level.is_some();

            let token = self.next_token;
            self.next_token += 1;

            let callback: IAudioEndpointVolumeCallback =
                VolumeNotifications { thread: self.thread, token }.into();

            if volume.RegisterControlChangeNotify(&callback).is_ok() {
                self.bindings.push(Binding {
                    token,
                    endpoint: endpoint.clone(),
                    volume,
                    callback,
                    adjustable,
                });
            }

            Some(Discovered {
                hardware: identity::hardware(&endpoint),
                is_default: default_endpoint == Some(endpoint.as_str()),
                name: friendly_name(device),
                endpoint,
                adjustable,
                level,
            })
        }
    }

    fn release_all(&mut self) {
        for binding in self.bindings.drain(..) {
            unsafe {
                let _ = binding.volume.UnregisterControlChangeNotify(&binding.callback);
            }
        }
    }

    fn on_volume_changed(&mut self, token: usize, observed: f32) {
        let Some(binding) = self.bindings.iter().find(|binding| binding.token == token) else {
            return;
        };

        self.state.set_level(&binding.endpoint, Some(observed));

        if let Some((target, locked)) = self.state.enforcement(&binding.endpoint) {
            if locked && (observed - target).abs() > TOLERANCE {
                write(binding, target);
            }
        }
    }

    fn enforce_all(&mut self) {
        for binding in &self.bindings {
            if !binding.adjustable {
                continue;
            }
            let Some((target, locked)) = self.state.enforcement(&binding.endpoint) else {
                continue;
            };
            if locked {
                write(binding, target);
                self.state.set_level(&binding.endpoint, Some(target));
            }
        }
    }

    // Catches change events the audio engine never delivered.
    fn poll(&mut self) {
        if self.bindings.is_empty() {
            self.refresh();
            return;
        }

        let mut stale = false;
        for binding in &self.bindings {
            if !binding.adjustable {
                continue;
            }
            let Ok(observed) = (unsafe { binding.volume.GetMasterVolumeLevelScalar() }) else {
                stale = true;
                continue;
            };

            self.state.set_level(&binding.endpoint, Some(observed));

            if let Some((target, locked)) = self.state.enforcement(&binding.endpoint) {
                if locked && (observed - target).abs() > TOLERANCE {
                    write(binding, target);
                }
            }
        }

        if stale {
            self.refresh();
        }
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.release_all();
        if let (Some(enumerator), Some(client)) = (&self.enumerator, &self.notifications) {
            unsafe {
                let _ = enumerator.UnregisterEndpointNotificationCallback(client);
            }
        }
    }
}

fn active_capture_devices(enumerator: &IMMDeviceEnumerator) -> Option<Vec<IMMDevice>> {
    unsafe {
        let collection = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE).ok()?;
        let count = collection.GetCount().ok()?;
        (0..count).map(|index| collection.Item(index).ok()).collect()
    }
}

fn write(binding: &Binding, target: f32) {
    unsafe {
        let _ = binding.volume.SetMasterVolumeLevelScalar(target, &EVENT_CONTEXT);
    }
}

fn endpoint_id(device: &IMMDevice) -> Option<String> {
    unsafe {
        let raw = device.GetId().ok()?;
        let id = raw.to_string().ok();
        CoTaskMemFree(Some(raw.0.cast()));
        id
    }
}

fn friendly_name(device: &IMMDevice) -> Option<String> {
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ).ok()?;
        let name = store.GetValue(&PKEY_Device_FriendlyName).ok()?.to_string();
        (!name.is_empty()).then_some(name)
    }
}

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeNotifications {
    thread: u32,
    token: usize,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeNotifications_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        let Some(data) = (unsafe { data.as_ref() }) else { return Ok(()) };
        if data.guidEventContext != EVENT_CONTEXT {
            post(
                self.thread,
                MSG_VOLUME,
                self.token,
                data.fMasterVolume.to_bits() as isize,
            );
        }
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DeviceNotifications {
    thread: u32,
}

impl IMMNotificationClient_Impl for DeviceNotifications_Impl {
    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, _role: ERole, _id: &PCWSTR) -> Result<()> {
        if flow == eCapture {
            post(self.thread, MSG_DEVICE, 0, 0);
        }
        Ok(())
    }

    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> Result<()> {
        post(self.thread, MSG_DEVICE, 0, 0);
        Ok(())
    }

    fn OnDeviceAdded(&self, _id: &PCWSTR) -> Result<()> {
        post(self.thread, MSG_DEVICE, 0, 0);
        Ok(())
    }

    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> Result<()> {
        post(self.thread, MSG_DEVICE, 0, 0);
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
