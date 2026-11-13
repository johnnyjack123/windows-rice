//! Wi-Fi, airplane mode and mobile hotspot (docs/BACKEND.md §4 "Netzwerk").

use crate::blocking::blocking;
use crate::bluetooth;
use crate::state::{AppState, Locked};
use rice_core::contract::{
    ConnectError, ConnectResult, HotspotState, NetworkState, WifiNetwork, WifiPermission, WifiState, events,
};
use rice_core::network::{
    ScannedNetwork, band_label, detail, merge_scan, profile_security, profile_xml, security_label, signal_bars,
};
use rice_core::{BackendError, BackendResult};
use rice_win::radios::{Kind, Radios};
use rice_win::wifi::{Available, Wlan, WlanError, WlanEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};
use windows_core::GUID;

const DEBOUNCE: Duration = Duration::from_millis(150);
const SCAN_TIMEOUT: Duration = Duration::from_secs(4);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// `DOT11_AUTH_ALGO_80211_OPEN`
const OPEN: i32 = 1;

/// Network service state.
#[derive(Default)]
pub struct Network {
    wlan: Mutex<Option<Arc<Wlan>>>,
    radios: Mutex<Option<Arc<Radios>>>,
    /// Radio states before airplane mode, restored when it ends.
    before_airplane: Mutex<Option<Vec<(Kind, bool)>>>,
    sent: Mutex<Option<NetworkState>>,
    scheduled: AtomicBool,
}

/// Opens WLAN and the radios in the background (both block briefly).
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        let events = handle.clone();
        match Wlan::open(move || schedule(&events)) {
            Ok(wlan) => *state.network.wlan.locked() = Some(Arc::new(wlan)),
            Err(error) => log::info!("wifi: no WLAN service ({error:?})"),
        }
        let events = handle.clone();
        match Radios::load(move || {
            schedule(&events);
            bluetooth::schedule(&events);
        }) {
            Ok(radios) => *state.network.radios.locked() = Some(Arc::new(radios)),
            Err(error) => log::warn!("radios: unavailable ({error})"),
        }
        let events = handle.clone();
        if let Err(error) = rice_win::radios::watch_network_status(move || schedule(&events)) {
            log::warn!("network: no status notifications ({error})");
        }
        schedule(&handle);
        bluetooth::schedule(&handle);
    });
}

/// The radios, shared with the Bluetooth service.
pub fn radios(app: &AppHandle) -> Option<Arc<Radios>> {
    app.state::<AppState>().network.radios.locked().clone()
}

fn wlan(app: &AppHandle) -> Option<(Arc<Wlan>, GUID)> {
    let wlan = app.state::<AppState>().network.wlan.locked().clone()?;
    let interface = wlan.interface()?;
    Some((wlan, interface))
}

/// Sends `network://state` once per burst of changes, if it changed.
pub fn schedule(app: &AppHandle) {
    if app.state::<AppState>().network.scheduled.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DEBOUNCE).await;
        handle.state::<AppState>().network.scheduled.store(false, Ordering::SeqCst);
        let refresh = handle.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let state = current(&refresh);
            let changed = refresh.state::<AppState>().network.sent.locked().replace(state.clone()).as_ref() != Some(&state);
            if changed {
                let _ = refresh.emit(events::NETWORK_STATE, state);
            }
        })
        .await;
    });
}

/// Builds the current state. Blocking.
fn current(app: &AppHandle) -> NetworkState {
    let radios = radios(app);
    let wlan = wlan(app);
    let enabled = radios.as_ref().map_or(wlan.is_some(), |r| r.is_on(Kind::WiFi));
    let available = wlan.is_some() || radios.as_ref().is_some_and(|r| r.has(Kind::WiFi));
    let mut permission = WifiPermission::Granted;
    let mut connected = None;
    if let Some((wlan, interface)) = &wlan
        && enabled
    {
        if wlan.available(interface) == Err(WlanError::AccessDenied) {
            permission = WifiPermission::LocationRequired;
        }
        connected = match wlan.current(interface) {
            Ok(Some(connection)) => {
                let (_, by_bssid) = wlan.frequencies(interface);
                let band = by_bssid.get(&connection.bssid).and_then(|f| band_label(*f, decimal_comma(app)));
                Some(WifiNetwork {
                    ssid: String::from_utf8_lossy(&connection.ssid).into_owned(),
                    signal: signal_bars(connection.quality),
                    secure: connection.auth != OPEN,
                    known: true,
                    detail: detail(band, security_label(connection.auth)),
                })
            }
            Ok(None) => None,
            Err(_) => {
                permission = WifiPermission::LocationRequired;
                // Without location access the network name still comes from the connection profile.
                rice_win::radios::internet_connection().map(|(ssid, bars)| WifiNetwork {
                    ssid,
                    signal: bars.unwrap_or(0).min(4),
                    secure: true,
                    known: true,
                    detail: String::new(),
                })
            }
        };
    }
    let hotspot = rice_win::radios::hotspot();
    NetworkState {
        wifi: WifiState { available, enabled, connected, permission },
        airplane: radios.as_ref().is_some_and(|r| r.all_off()),
        hotspot: HotspotState { supported: hotspot.is_some(), enabled: hotspot.unwrap_or(false) },
    }
}

/// Decimal comma for "2,4 GHz", following the app language.
fn decimal_comma(app: &AppHandle) -> bool {
    match app.state::<AppState>().settings.locked().locale.as_str() {
        "de" => true,
        "en" => false,
        _ => rice_win::locale::ui_is_german(),
    }
}

/// `network_get`.
pub async fn get(app: &AppHandle) -> BackendResult<NetworkState> {
    let handle = app.clone();
    blocking(move || {
        let state = current(&handle);
        *handle.state::<AppState>().network.sent.locked() = Some(state.clone());
        Ok(state)
    })
    .await
}

fn permission_error() -> BackendError {
    BackendError::new(rice_core::ErrorCode::PermissionRequired, "location access required for Wi-Fi details")
}

fn wlan_error(error: WlanError) -> BackendError {
    match error {
        WlanError::AccessDenied => permission_error(),
        WlanError::NotFound => BackendError::not_found("no such network"),
        WlanError::Other(code) => BackendError::internal(format!("WLAN error {code}")),
    }
}

/// Waits for the first event `pick` accepts.
fn wait_for<T>(events: &Receiver<WlanEvent>, timeout: Duration, mut pick: impl FnMut(&WlanEvent) -> Option<T>) -> Option<T> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.checked_duration_since(Instant::now())?;
        let event = events.recv_timeout(left).ok()?;
        if let Some(result) = pick(&event) {
            return Some(result);
        }
    }
}

/// `network_scan`: fresh list, strongest entry per SSID, without the connected one.
pub async fn scan(app: &AppHandle) -> BackendResult<Vec<WifiNetwork>> {
    let handle = app.clone();
    let list = blocking(move || {
        let Some((wlan, interface)) = wlan(&handle) else {
            return Ok(Vec::new());
        };
        if !radios(&handle).is_none_or(|r| r.is_on(Kind::WiFi)) {
            return Ok(Vec::new());
        }
        let events = wlan.subscribe();
        wlan.scan(&interface).map_err(wlan_error)?;
        let finished = wait_for(&events, SCAN_TIMEOUT, |e| matches!(e, WlanEvent::ScanComplete | WlanEvent::ScanFailed).then_some(()));
        if finished.is_none() {
            log::debug!("wifi: scan did not finish in time; using what Windows has");
        }
        let networks = wlan.available(&interface).map_err(wlan_error)?;
        let (by_ssid, _) = wlan.frequencies(&interface);
        let comma = decimal_comma(&handle);
        let connected = networks.iter().find(|n| n.connected).map(|n| String::from_utf8_lossy(&n.ssid).into_owned());
        let entries = networks
            .iter()
            .map(|n| ScannedNetwork {
                ssid: String::from_utf8_lossy(&n.ssid).into_owned(),
                quality: n.quality,
                secure: n.secure,
                known: n.has_profile,
                detail: detail(by_ssid.get(&n.ssid).and_then(|f| band_label(*f, comma)), security_label(n.auth)),
            })
            .collect();
        Ok(merge_scan(entries, connected.as_deref()))
    })
    .await?;
    let _ = app.emit(events::NETWORK_SCAN, list.clone());
    Ok(list)
}

/// `network_connect`: known networks by profile, new ones with a fresh profile.
pub async fn connect(app: &AppHandle, ssid: String, key: Option<String>) -> BackendResult<ConnectResult> {
    let handle = app.clone();
    let result = blocking(move || {
        let (wlan, interface) = wlan(&handle).ok_or_else(|| BackendError::not_supported("no Wi-Fi adapter"))?;
        let networks = wlan.available(&interface).map_err(wlan_error)?;
        let matching: Vec<&Available> = networks.iter().filter(|n| String::from_utf8_lossy(&n.ssid) == ssid).collect();
        let Some(network) = matching.iter().max_by_key(|n| n.quality) else {
            return Err(BackendError::not_found(format!("network {ssid} is not in range")));
        };
        let known = matching.iter().find(|n| n.has_profile && !n.profile.is_empty());
        let (profile, created) = match (known, key.as_deref().filter(|k| !k.is_empty())) {
            (Some(known), None) => (known.profile.clone(), false),
            (_, key) => {
                let Some(security) = profile_security(network.auth) else {
                    log::info!("wifi: {ssid} needs enterprise credentials; use Windows' settings");
                    return Ok(ConnectResult::failed(ConnectError::Failed));
                };
                if network.secure && key.is_none() {
                    return Ok(ConnectResult::failed(ConnectError::WrongKey));
                }
                let xml = profile_xml(&network.ssid, &ssid, security, key);
                if let Err(error) = wlan.set_profile(&interface, &xml) {
                    log::warn!("wifi: profile for {ssid} rejected ({error:?})");
                    return Ok(ConnectResult::failed(ConnectError::Failed));
                }
                (ssid.clone(), true)
            }
        };
        let events = wlan.subscribe();
        wlan.connect(&interface, &profile).map_err(wlan_error)?;
        let outcome = wait_for(&events, CONNECT_TIMEOUT, |event| match event {
            WlanEvent::ConnectionComplete { profile: p, .. } if *p == profile => Some(Ok(())),
            WlanEvent::ConnectionFailed { profile: p, reason } if *p == profile => Some(Err(*reason)),
            _ => None,
        });
        Ok(match outcome {
            Some(Ok(())) => ConnectResult::OK,
            Some(Err(reason)) => {
                let error = rice_core::network::connect_error(reason);
                log::info!("wifi: connecting to {ssid} failed (reason {reason:#x})");
                // A profile with a wrong key would make Windows retry it forever.
                if created {
                    wlan.delete_profile(&interface, &profile);
                }
                ConnectResult::failed(error)
            }
            None => ConnectResult::failed(ConnectError::Timeout),
        })
    })
    .await;
    schedule(app);
    result
}

/// `network_disconnect`.
pub async fn disconnect(app: &AppHandle) -> BackendResult<()> {
    let handle = app.clone();
    blocking(move || {
        let (wlan, interface) = wlan(&handle).ok_or_else(|| BackendError::not_supported("no Wi-Fi adapter"))?;
        wlan.disconnect(&interface).map_err(wlan_error)
    })
    .await
}

fn switch(radios: &Radios, kind: Kind, on: bool) -> BackendResult<()> {
    match radios.set(kind, on) {
        Ok(true) => Ok(()),
        Ok(false) => Err(BackendError::access_denied("Windows did not allow switching the radio")),
        Err(error) => Err(BackendError::internal(error)),
    }
}

/// `network_set_wifi`. Turning Wi-Fi on also ends airplane mode.
pub async fn set_wifi(app: &AppHandle, enabled: bool) -> BackendResult<()> {
    let radios = radios(app).ok_or_else(|| BackendError::not_supported("radios unavailable"))?;
    if enabled {
        *app.state::<AppState>().network.before_airplane.locked() = None;
    }
    blocking(move || switch(&radios, Kind::WiFi, enabled)).await
}

/// `network_set_airplane`: all radios off (remembering their states), or back.
pub async fn set_airplane(app: &AppHandle, enabled: bool) -> BackendResult<()> {
    let radios = radios(app).ok_or_else(|| BackendError::not_supported("radios unavailable"))?;
    let saved = {
        let network = &app.state::<AppState>().network;
        let mut before = network.before_airplane.locked();
        if enabled {
            *before = Some(radios.states());
            None
        } else {
            before.take()
        }
    };
    blocking(move || {
        if enabled {
            for kind in [Kind::WiFi, Kind::Bluetooth, Kind::MobileBroadband].into_iter().filter(|k| radios.has(*k)) {
                switch(&radios, kind, false)?;
            }
            return Ok(());
        }
        let restore = saved.unwrap_or_else(|| vec![(Kind::WiFi, true), (Kind::Bluetooth, true)]);
        for (kind, on) in restore.into_iter().filter(|(k, on)| *on && radios.has(*k)) {
            switch(&radios, kind, on)?;
        }
        Ok(())
    })
    .await
}

/// `network_set_hotspot`.
pub async fn set_hotspot(app: &AppHandle, enabled: bool) -> BackendResult<()> {
    let result = blocking(move || match rice_win::radios::set_hotspot(enabled) {
        Ok(true) => Ok(()),
        Ok(false) => Err(BackendError::not_supported("the mobile hotspot cannot be switched here")),
        Err(error) => Err(BackendError::internal(error)),
    })
    .await;
    schedule(app);
    result
}
