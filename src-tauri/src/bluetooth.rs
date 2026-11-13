//! Bluetooth radio and paired devices (docs/BACKEND.md §4 "Bluetooth").

use crate::blocking::blocking;
use crate::network;
use crate::state::{AppState, Locked};
use rice_core::contract::{BluetoothDevice, BluetoothState, ConnectError, ConnectResult, events};
use rice_core::network::bluetooth_kind;
use rice_core::{BackendError, BackendResult};
use rice_win::bluetooth::{Paired, Watchers};
use rice_win::radios::Kind;
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows_core::GUID;

const DEBOUNCE: Duration = Duration::from_millis(300);
/// Audio filters change only when devices are paired or removed.
const SWITCHABLE_MAX_AGE: Duration = Duration::from_secs(60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL: Duration = Duration::from_millis(500);

/// Bluetooth service state.
#[derive(Default)]
pub struct Bluetooth {
    watchers: Mutex<Option<Watchers>>,
    switchable: Mutex<Option<(Instant, HashSet<GUID>)>>,
    sent: Mutex<Option<BluetoothState>>,
    scheduled: AtomicBool,
}

/// Watches paired devices in the background.
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let events = handle.clone();
        match rice_win::bluetooth::watch(move || schedule(&events)) {
            Ok(watchers) => *handle.state::<AppState>().bluetooth.watchers.locked() = Some(watchers),
            Err(error) => log::warn!("bluetooth: no device notifications ({error})"),
        }
    });
}

/// Sends `bluetooth://state` once per burst of changes, if it changed.
pub fn schedule(app: &AppHandle) {
    if app.state::<AppState>().bluetooth.scheduled.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DEBOUNCE).await;
        handle.state::<AppState>().bluetooth.scheduled.store(false, Ordering::SeqCst);
        let refresh = handle.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let state = current(&refresh);
            let changed = refresh.state::<AppState>().bluetooth.sent.locked().replace(state.clone()).as_ref() != Some(&state);
            if changed {
                let _ = refresh.emit(events::BLUETOOTH_STATE, state);
            }
        })
        .await;
    });
}

/// Audio devices that can be connected from here (cached; the lookup walks
/// every audio endpoint).
fn switchable(app: &AppHandle) -> HashSet<GUID> {
    let bluetooth = &app.state::<AppState>().bluetooth;
    if let Some((at, set)) = &*bluetooth.switchable.locked()
        && at.elapsed() < SWITCHABLE_MAX_AGE
    {
        return set.clone();
    }
    let set = rice_win::bluetooth::switchable_containers();
    *bluetooth.switchable.locked() = Some((Instant::now(), set.clone()));
    set
}

fn device(paired: &Paired, switchable: &HashSet<GUID>) -> BluetoothDevice {
    // Windows keeps the last known level of disconnected devices; show it only while connected.
    let battery = paired
        .container
        .filter(|_| paired.connected)
        .and_then(rice_win::bluetooth::battery)
        .map(|percent| f64::from(percent) / 100.0);
    BluetoothDevice {
        id: paired.id.clone(),
        name: paired.name.trim().to_owned(),
        kind: bluetooth_kind(&paired.name, paired.hints),
        connected: paired.connected,
        battery,
        can_connect: paired.container.is_some_and(|c| switchable.contains(&c)),
    }
}

/// Builds the current state. Blocking.
fn current(app: &AppHandle) -> BluetoothState {
    let radios = network::radios(app);
    let available = radios.as_ref().is_some_and(|r| r.has(Kind::Bluetooth));
    let enabled = radios.as_ref().is_some_and(|r| r.is_on(Kind::Bluetooth));
    let paired = rice_win::bluetooth::paired().unwrap_or_else(|error| {
        log::warn!("bluetooth: paired devices unavailable ({error})");
        Vec::new()
    });
    let switchable = if paired.is_empty() { HashSet::new() } else { switchable(app) };
    let mut devices: Vec<BluetoothDevice> = paired.iter().map(|p| device(p, &switchable)).collect();
    devices.sort_by(|a, b| b.connected.cmp(&a.connected).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    BluetoothState { available, enabled, devices }
}

/// `bluetooth_get`.
pub async fn get(app: &AppHandle) -> BackendResult<BluetoothState> {
    let handle = app.clone();
    blocking(move || {
        let state = current(&handle);
        *handle.state::<AppState>().bluetooth.sent.locked() = Some(state.clone());
        Ok(state)
    })
    .await
}

/// `bluetooth_set_enabled`.
pub async fn set_enabled(app: &AppHandle, enabled: bool) -> BackendResult<()> {
    let radios = network::radios(app).ok_or_else(|| BackendError::not_supported("radios unavailable"))?;
    blocking(move || match radios.set(Kind::Bluetooth, enabled) {
        Ok(true) => Ok(()),
        Ok(false) => Err(BackendError::access_denied("Windows did not allow switching Bluetooth")),
        Err(error) => Err(BackendError::internal(error)),
    })
    .await
}

fn find(id: &str) -> BackendResult<Paired> {
    rice_win::bluetooth::paired()
        .map_err(BackendError::internal)?
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| BackendError::not_found(format!("no paired device {id}")))
}

/// Polls until the device reaches the wanted connection state.
fn wait_connected(id: &str, wanted: bool) -> bool {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    while Instant::now() < deadline {
        if find(id).is_ok_and(|d| d.connected == wanted) {
            return true;
        }
        thread::sleep(POLL);
    }
    false
}

/// `bluetooth_connect`: audio devices only; others report `failed` (the UI
/// sends those to Windows' settings).
pub async fn connect(app: &AppHandle, id: String) -> BackendResult<ConnectResult> {
    let handle = app.clone();
    let result = blocking(move || {
        let device = find(&id)?;
        if device.connected {
            return Ok(ConnectResult::OK);
        }
        let Some(container) = device.container.filter(|c| switchable(&handle).contains(c)) else {
            return Ok(ConnectResult::failed(ConnectError::Failed));
        };
        if !rice_win::bluetooth::switch_audio(container, true) {
            return Ok(ConnectResult::failed(ConnectError::Failed));
        }
        Ok(if wait_connected(&id, true) { ConnectResult::OK } else { ConnectResult::failed(ConnectError::Timeout) })
    })
    .await;
    schedule(app);
    result
}

/// `bluetooth_disconnect`.
pub async fn disconnect(app: &AppHandle, id: String) -> BackendResult<()> {
    let handle = app.clone();
    let result = blocking(move || {
        let device = find(&id)?;
        let container = device
            .container
            .filter(|c| switchable(&handle).contains(c))
            .ok_or_else(|| BackendError::not_supported("this device can only be disconnected in Windows' settings"))?;
        if !rice_win::bluetooth::switch_audio(container, false) {
            return Err(BackendError::internal("the audio driver refused to disconnect"));
        }
        wait_connected(&id, false);
        Ok(())
    })
    .await;
    schedule(app);
    result
}
