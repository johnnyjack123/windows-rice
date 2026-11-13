//! Core Audio: default output/input, volume, mute and device list.
//!
//! Core Audio interfaces are not thread-safe in windows-rs, so one thread owns
//! all of them. Commands send it jobs; volume callbacks (including the
//! hardware volume keys) and device notifications wake it up, and it reports
//! a fresh snapshot once per burst.

use crate::com::ComGuard;
use rice_core::contract::AudioDeviceKind;
use rice_core::devices::audio_kind;
use rice_core::{BackendError, BackendResult};
use std::collections::HashMap;
use std::ptr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, DEVICE_STATE_ACTIVE, EDataFlow, ERole, IDeviceTopology, IMMDevice,
    IMMDeviceEnumerator, IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, PKEY_AudioEndpoint_FormFactor,
    eCapture, eConsole, eRender,
};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToStringAlloc, PropVariantToUInt32};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::{PCWSTR, PWSTR, implement};

/// One output device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub id: String,
    pub name: String,
    pub kind: AudioDeviceKind,
}

/// Everything `audio_get` reports.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioSnapshot {
    pub volume: f32,
    pub muted: bool,
    pub device: Option<Endpoint>,
    pub devices: Vec<Endpoint>,
    pub mic_muted: bool,
    pub mic_available: bool,
}

type Job = Box<dyn FnOnce(&mut Worker) + Send>;

enum Message {
    Job(Job),
    /// Volume or mute of a bound endpoint changed.
    Volume,
    /// Devices were added, removed or a default changed.
    Devices,
}

/// Handle to the audio thread; cheap to clone.
#[derive(Clone)]
pub struct AudioHandle {
    tx: Sender<Message>,
}

impl AudioHandle {
    /// Starts the audio thread. `on_change` runs on it after every change.
    pub fn start(on_change: impl Fn(AudioSnapshot) + Send + 'static) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let events = tx.clone();
        thread::Builder::new().name("rice-audio".into()).spawn(move || run(events, rx, on_change))?;
        Ok(Self { tx })
    }

    fn call<R: Send + 'static>(&self, job: impl FnOnce(&mut Worker) -> R + Send + 'static) -> BackendResult<R> {
        let (reply, result) = mpsc::channel();
        self.tx
            .send(Message::Job(Box::new(move |worker| {
                let _ = reply.send(job(worker));
            })))
            .map_err(|_| BackendError::not_supported("audio is unavailable"))?;
        result.recv().map_err(|_| BackendError::not_supported("audio is unavailable"))
    }

    /// Current state (blocks until the audio thread answers).
    pub fn snapshot(&self) -> BackendResult<AudioSnapshot> {
        self.call(|worker| worker.snapshot())
    }

    /// Sets the output volume (0…1) and unmutes.
    pub fn set_volume(&self, volume: f32) -> BackendResult<()> {
        self.call(move |worker| worker.set_volume(volume))?
    }

    /// Mutes or unmutes the output.
    pub fn set_muted(&self, muted: bool) -> BackendResult<()> {
        self.call(move |worker| worker.set_mute(eRender, muted))?
    }

    /// Mutes or unmutes the default microphone.
    pub fn set_mic_muted(&self, muted: bool) -> BackendResult<()> {
        self.call(move |worker| worker.set_mute(eCapture, muted))?
    }

    /// Makes `id` the default output for every role.
    pub fn set_default_device(&self, id: String) -> BackendResult<()> {
        self.call(move |worker| worker.set_default(&id))?
    }
}

fn run(events: Sender<Message>, rx: Receiver<Message>, on_change: impl Fn(AudioSnapshot)) {
    let _com = ComGuard::mta();
    let mut worker = match Worker::new(events) {
        Ok(worker) => worker,
        Err(error) => {
            // Pending and future jobs see a closed channel → not_supported.
            log::warn!("audio: unavailable ({error})");
            return;
        }
    };
    while let Ok(first) = rx.recv() {
        let (mut volume, mut devices) = (false, false);
        let mut handle = |message: Message, worker: &mut Worker| match message {
            Message::Job(job) => job(worker),
            Message::Volume => volume = true,
            Message::Devices => devices = true,
        };
        handle(first, &mut worker);
        // Coalesce a burst (volume keys, device switches) into one report.
        while let Ok(next) = rx.try_recv() {
            handle(next, &mut worker);
        }
        if devices {
            worker.rebind();
        }
        if volume || devices {
            on_change(worker.snapshot());
        }
    }
}

struct Bound {
    id: String,
    volume: IAudioEndpointVolume,
}

struct Worker {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
    callback: IAudioEndpointVolumeCallback,
    render: Option<Bound>,
    capture: Option<Bound>,
    /// Device kinds need a topology lookup; they never change for an id.
    kinds: HashMap<String, AudioDeviceKind>,
}

impl Worker {
    fn new(events: Sender<Message>) -> windows::core::Result<Self> {
        // SAFETY: COM calls on this MTA thread.
        let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let client: IMMNotificationClient = DeviceEvents { tx: events.clone() }.into();
        // SAFETY: the client stays registered for the life of the worker.
        unsafe { enumerator.RegisterEndpointNotificationCallback(&client)? };
        let callback: IAudioEndpointVolumeCallback = VolumeEvents { tx: events }.into();
        let mut worker =
            Self { enumerator, client, callback, render: None, capture: None, kinds: HashMap::new() };
        worker.rebind();
        Ok(worker)
    }

    /// Follows the default devices: volume callbacks move to the new endpoints.
    fn rebind(&mut self) {
        for bound in [self.render.take(), self.capture.take()].into_iter().flatten() {
            // SAFETY: unregisters the callback registered in `bind`.
            let _ = unsafe { bound.volume.UnregisterControlChangeNotify(&self.callback) };
        }
        self.render = self.bind(eRender);
        self.capture = self.bind(eCapture);
    }

    fn bind(&self, flow: EDataFlow) -> Option<Bound> {
        // SAFETY: COM calls on this thread; the device may be gone (→ None).
        unsafe {
            let device = self.enumerator.GetDefaultAudioEndpoint(flow, eConsole).ok()?;
            let id = device_id(&device)?;
            let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
            if let Err(error) = volume.RegisterControlChangeNotify(&self.callback) {
                log::warn!("audio: no volume notifications for {id}: {error}");
            }
            Some(Bound { id, volume })
        }
    }

    fn bound(&self, flow: EDataFlow) -> Option<&Bound> {
        if flow == eCapture { self.capture.as_ref() } else { self.render.as_ref() }
    }

    fn snapshot(&mut self) -> AudioSnapshot {
        let (volume, muted) = self
            .render
            .as_ref()
            // SAFETY: reads on a live endpoint owned by this thread.
            .map(|b| unsafe {
                (b.volume.GetMasterVolumeLevelScalar().unwrap_or(0.0), b.volume.GetMute().is_ok_and(|m| m.as_bool()))
            })
            .unwrap_or((0.0, false));
        let devices = self.render_devices();
        let device = self.render.as_ref().and_then(|b| devices.iter().find(|d| d.id == b.id).cloned());
        // SAFETY: read on a live endpoint owned by this thread.
        let mic_muted = self.capture.as_ref().is_some_and(|b| unsafe { b.volume.GetMute().is_ok_and(|m| m.as_bool()) });
        AudioSnapshot { volume, muted, device, devices, mic_muted, mic_available: self.capture.is_some() }
    }

    fn render_devices(&mut self) -> Vec<Endpoint> {
        // SAFETY: enumeration on this thread.
        let Ok(collection) = (unsafe { self.enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }) else {
            return Vec::new();
        };
        let count = unsafe { collection.GetCount() }.unwrap_or(0);
        (0..count)
            // SAFETY: index below the collection's count.
            .filter_map(|i| unsafe { collection.Item(i) }.ok())
            .filter_map(|device| self.describe(&device))
            .collect()
    }

    fn describe(&mut self, device: &IMMDevice) -> Option<Endpoint> {
        let id = device_id(device)?;
        // SAFETY: read-only property store of a live device.
        let store = unsafe { device.OpenPropertyStore(STGM_READ) }.ok()?;
        let name = property_string(&store, &PKEY_Device_FriendlyName).unwrap_or_else(|| id.clone());
        let kind = match self.kinds.get(&id) {
            Some(kind) => *kind,
            None => {
                let form_factor = property_u32(&store, &PKEY_AudioEndpoint_FormFactor);
                let kind = audio_kind(connected_device_id(device).as_deref(), form_factor);
                self.kinds.insert(id.clone(), kind);
                kind
            }
        };
        Some(Endpoint { id, name, kind })
    }

    fn set_volume(&self, volume: f32) -> BackendResult<()> {
        let bound = self.render.as_ref().ok_or_else(|| BackendError::not_found("no output device"))?;
        // SAFETY: writes on a live endpoint owned by this thread.
        unsafe {
            bound.volume.SetMasterVolumeLevelScalar(volume.clamp(0.0, 1.0), ptr::null()).map_err(BackendError::internal)?;
            bound.volume.SetMute(false, ptr::null()).map_err(BackendError::internal)
        }
    }

    fn set_mute(&self, flow: EDataFlow, muted: bool) -> BackendResult<()> {
        let bound = self.bound(flow).ok_or_else(|| BackendError::not_found("no such device"))?;
        // SAFETY: write on a live endpoint owned by this thread.
        unsafe { bound.volume.SetMute(muted, ptr::null()) }.map_err(BackendError::internal)
    }

    #[cfg(feature = "policy-config")]
    fn set_default(&self, id: &str) -> BackendResult<()> {
        let known = self.enumerator_has_device(id);
        if !known {
            return Err(BackendError::not_found(format!("no output device {id}")));
        }
        policy::set_default_endpoint(id).map_err(|e| BackendError::not_supported(format!("cannot switch devices: {e}")))
    }

    #[cfg(not(feature = "policy-config"))]
    fn set_default(&self, _id: &str) -> BackendResult<()> {
        Err(BackendError::not_supported("switching the default device is disabled in this build"))
    }

    #[cfg(feature = "policy-config")]
    fn enumerator_has_device(&self, id: &str) -> bool {
        let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
        // SAFETY: NUL-terminated id; the device is only probed.
        unsafe { self.enumerator.GetDevice(PCWSTR(wide.as_ptr())) }
            .and_then(|d| unsafe { d.GetState() })
            .is_ok_and(|state| state == DEVICE_STATE_ACTIVE)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        for bound in [self.render.take(), self.capture.take()].into_iter().flatten() {
            // SAFETY: balances the registrations of this worker.
            let _ = unsafe { bound.volume.UnregisterControlChangeNotify(&self.callback) };
        }
        // SAFETY: balances the registration in `new`.
        let _ = unsafe { self.enumerator.UnregisterEndpointNotificationCallback(&self.client) };
    }
}

fn device_id(device: &IMMDevice) -> Option<String> {
    // SAFETY: the returned string is CoTaskMem-allocated and freed by take_string.
    unsafe { device.GetId().ok().map(|raw| take_string(raw)) }
}

/// Id of the driver device behind an endpoint, e.g. `{2}.\\?\bthenum#…`.
fn connected_device_id(device: &IMMDevice) -> Option<String> {
    // SAFETY: topology queries on a live device; the id string is freed.
    unsafe {
        let topology: IDeviceTopology = device.Activate(CLSCTX_ALL, None).ok()?;
        let connector = topology.GetConnector(0).ok()?;
        connector.GetDeviceIdConnectedTo().ok().map(|raw| take_string(raw))
    }
}

fn property_string(store: &windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore, key: &PROPERTYKEY) -> Option<String> {
    // SAFETY: the PROPVARIANT and the converted string are both released.
    unsafe {
        let mut value = store.GetValue(key).ok()?;
        let text = PropVariantToStringAlloc(&value).ok().map(|raw| take_string(raw));
        let _ = PropVariantClear(&mut value);
        text.filter(|t| !t.is_empty())
    }
}

fn property_u32(store: &windows::Win32::UI::Shell::PropertiesSystem::IPropertyStore, key: &PROPERTYKEY) -> Option<u32> {
    // SAFETY: the PROPVARIANT is released after conversion.
    unsafe {
        let mut value = store.GetValue(key).ok()?;
        let number = PropVariantToUInt32(&value).ok();
        let _ = PropVariantClear(&mut value);
        number
    }
}

/// Copies and frees a CoTaskMem string.
///
/// # Safety
/// `raw` must be null or allocated with `CoTaskMemAlloc`.
unsafe fn take_string(raw: PWSTR) -> String {
    if raw.is_null() {
        return String::new();
    }
    // SAFETY: guaranteed by the caller.
    unsafe {
        let text = raw.to_string().unwrap_or_default();
        CoTaskMemFree(Some(raw.0 as *const _));
        text
    }
}

/// Device notifications → wake the audio thread.
#[implement(IMMNotificationClient)]
struct DeviceEvents {
    tx: Sender<Message>,
}

impl IMMNotificationClient_Impl for DeviceEvents_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> windows::core::Result<()> {
        let _ = self.tx.send(Message::Devices);
        Ok(())
    }

    fn OnDeviceAdded(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.tx.send(Message::Devices);
        Ok(())
    }

    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> windows::core::Result<()> {
        let _ = self.tx.send(Message::Devices);
        Ok(())
    }

    fn OnDefaultDeviceChanged(&self, _flow: EDataFlow, role: ERole, _id: &PCWSTR) -> windows::core::Result<()> {
        // One notification per role; the console role is the one we follow.
        if role == eConsole {
            let _ = self.tx.send(Message::Devices);
        }
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _id: &PCWSTR, key: &PROPERTYKEY) -> windows::core::Result<()> {
        // Renamed devices; every other property changes far too often.
        if *key == PKEY_Device_FriendlyName {
            let _ = self.tx.send(Message::Devices);
        }
        Ok(())
    }
}

/// Volume and mute changes (also hardware keys) → wake the audio thread.
#[implement(IAudioEndpointVolumeCallback)]
struct VolumeEvents {
    tx: Sender<Message>,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeEvents_Impl {
    fn OnNotify(&self, _data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        let _ = self.tx.send(Message::Volume);
        Ok(())
    }
}

/// The undocumented `IPolicyConfig` interface that Windows' own sound
/// settings use to change the default device.
#[cfg(feature = "policy-config")]
// COM method names follow the Windows SDK.
#[allow(non_snake_case)]
mod policy {
    use windows::Win32::Media::Audio::{ERole, eCommunications, eConsole, eMultimedia};
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
    use windows::core::{GUID, HRESULT, IUnknown, IUnknown_Vtbl, PCWSTR, interface};

    const POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

    // Only the slot order matters; the methods before SetDefaultEndpoint are never called.
    #[interface("f8679f50-850a-41cf-9c72-430f290290c8")]
    unsafe trait IPolicyConfig: IUnknown {
        fn GetMixFormat(&self) -> HRESULT;
        fn GetDeviceFormat(&self) -> HRESULT;
        fn ResetDeviceFormat(&self) -> HRESULT;
        fn SetDeviceFormat(&self) -> HRESULT;
        fn GetProcessingPeriod(&self) -> HRESULT;
        fn SetProcessingPeriod(&self) -> HRESULT;
        fn GetShareMode(&self) -> HRESULT;
        fn SetShareMode(&self) -> HRESULT;
        fn GetPropertyValue(&self) -> HRESULT;
        fn SetPropertyValue(&self) -> HRESULT;
        fn SetDefaultEndpoint(&self, device_id: PCWSTR, role: ERole) -> HRESULT;
        fn SetEndpointVisibility(&self) -> HRESULT;
    }

    pub(super) fn set_default_endpoint(id: &str) -> windows::core::Result<()> {
        let wide: Vec<u16> = id.encode_utf16().chain(Some(0)).collect();
        // SAFETY: COM object of the audio service; NUL-terminated id.
        unsafe {
            let config: IPolicyConfig = CoCreateInstance(&POLICY_CONFIG_CLIENT, None, CLSCTX_ALL)?;
            for role in [eConsole, eMultimedia, eCommunications] {
                config.SetDefaultEndpoint(PCWSTR(wide.as_ptr()), role).ok()?;
            }
        }
        Ok(())
    }
}
