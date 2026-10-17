//! Native Wi-Fi (`wlanapi`): connection, available networks, scanning,
//! profiles and notifications.
//!
//! Since Windows 11 24H2 the calls that reveal SSIDs fail with
//! `ERROR_ACCESS_DENIED` unless the user lets desktop apps use the location;
//! that surfaces as [`WlanError::AccessDenied`].

use crate::from_wide;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_NOT_FOUND, ERROR_SUCCESS, HANDLE};
use windows::Win32::NetworkManagement::WiFi::{
    DOT11_SSID, L2_NOTIFICATION_DATA, WLAN_AVAILABLE_NETWORK, WLAN_AVAILABLE_NETWORK_CONNECTED,
    WLAN_AVAILABLE_NETWORK_HAS_PROFILE, WLAN_AVAILABLE_NETWORK_LIST, WLAN_BSS_ENTRY, WLAN_BSS_LIST,
    WLAN_CONNECTION_ATTRIBUTES, WLAN_CONNECTION_NOTIFICATION_DATA, WLAN_CONNECTION_PARAMETERS, WLAN_INTERFACE_INFO,
    WLAN_INTERFACE_INFO_LIST, WLAN_NOTIFICATION_SOURCE_ACM, WLAN_NOTIFICATION_SOURCE_MSM, WLAN_PROFILE_USER,
    WlanCloseHandle, WlanConnect, WlanDeleteProfile, WlanDisconnect, WlanEnumInterfaces, WlanFreeMemory,
    WlanGetAvailableNetworkList, WlanGetNetworkBssList, WlanOpenHandle, WlanQueryInterface, WlanRegisterNotification,
    WlanScan, WlanSetProfile, dot11_BSS_type_infrastructure, wlan_connection_mode_profile,
    wlan_intf_opcode_current_connection, wlan_notification_acm_connection_attempt_fail,
    wlan_notification_acm_connection_complete, wlan_notification_acm_disconnected, wlan_notification_acm_interface_arrival,
    wlan_notification_acm_interface_removal, wlan_notification_acm_scan_complete, wlan_notification_acm_scan_fail,
    wlan_notification_msm_connected, wlan_notification_msm_disconnected, wlan_notification_msm_radio_state_change,
};
use windows::core::{GUID, HSTRING, PCWSTR};

/// Client version 2 (Vista and later).
const CLIENT_VERSION: u32 = 2;

/// Why a WLAN call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WlanError {
    /// Location access is off for desktop apps (Windows 11 24H2+).
    AccessDenied,
    NotFound,
    Other(u32),
}

impl WlanError {
    fn from_code(code: u32) -> Self {
        match code {
            c if c == ERROR_ACCESS_DENIED.0 => Self::AccessDenied,
            c if c == ERROR_NOT_FOUND.0 => Self::NotFound,
            c => Self::Other(c),
        }
    }
}

fn check(code: u32) -> Result<(), WlanError> {
    if code == ERROR_SUCCESS.0 { Ok(()) } else { Err(WlanError::from_code(code)) }
}

/// Notifications the backend reacts to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WlanEvent {
    ScanComplete,
    ScanFailed,
    /// A connection attempt finished; `reason` 0 means success.
    ConnectionComplete { profile: String, reason: u32 },
    ConnectionFailed { profile: String, reason: u32 },
    /// Connection, radio or interface state changed.
    Changed,
}

/// The current connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub ssid: Vec<u8>,
    pub profile: String,
    pub quality: u32,
    pub auth: i32,
    pub bssid: [u8; 6],
}

/// One entry of the available-network list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Available {
    pub ssid: Vec<u8>,
    pub profile: String,
    pub quality: u32,
    pub secure: bool,
    pub auth: i32,
    pub connected: bool,
    pub has_profile: bool,
}

type Listeners = Mutex<Vec<Sender<WlanEvent>>>;

struct Context {
    on_change: Box<dyn Fn() + Send + Sync>,
    listeners: Listeners,
}

/// An open WLAN client handle with notifications.
pub struct Wlan {
    handle: HANDLE,
    context: Arc<Context>,
}

// SAFETY: WLAN client handles may be used from any thread; the context is Sync.
unsafe impl Send for Wlan {}
// SAFETY: as above.
unsafe impl Sync for Wlan {}

impl Wlan {
    /// Opens a client handle; `on_change` runs (on a system thread) whenever
    /// the connection, radio or interfaces change.
    pub fn open(on_change: impl Fn() + Send + Sync + 'static) -> Result<Self, WlanError> {
        let mut negotiated = 0u32;
        let mut handle = HANDLE::default();
        // SAFETY: out-pointers to locals.
        check(unsafe { WlanOpenHandle(CLIENT_VERSION, None, &mut negotiated, &mut handle) })?;
        let context = Arc::new(Context { on_change: Box::new(on_change), listeners: Mutex::new(Vec::new()) });
        // The registration lives as long as the handle; the context outlives both.
        let raw = Arc::into_raw(Arc::clone(&context));
        // SAFETY: valid handle; the callback only reads the context.
        let status = unsafe {
            WlanRegisterNotification(
                handle,
                WLAN_NOTIFICATION_SOURCE_ACM | WLAN_NOTIFICATION_SOURCE_MSM,
                true,
                Some(notification),
                Some(raw.cast()),
                None,
                None,
            )
        };
        if let Err(error) = check(status) {
            log::warn!("wifi: no notifications ({error:?})");
        }
        Ok(Self { handle, context })
    }

    /// Receives every notification from now on (scan and connect waits).
    pub fn subscribe(&self) -> Receiver<WlanEvent> {
        let (tx, rx) = mpsc::channel();
        self.context.listeners.lock().unwrap_or_else(PoisonError::into_inner).push(tx);
        rx
    }

    /// The first Wi-Fi interface, if the machine has one.
    pub fn interface(&self) -> Option<GUID> {
        let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
        // SAFETY: the list is allocated by wlanapi and freed below.
        unsafe {
            check(WlanEnumInterfaces(self.handle, None, &mut list)).ok()?;
            let items: &[WLAN_INTERFACE_INFO] =
                std::slice::from_raw_parts((*list).InterfaceInfo.as_ptr(), (*list).dwNumberOfItems as usize);
            let guid = items.first().map(|i| i.InterfaceGuid);
            WlanFreeMemory(list.cast());
            guid
        }
    }

    /// The current connection; `Ok(None)` when disconnected.
    pub fn current(&self, interface: &GUID) -> Result<Option<Connection>, WlanError> {
        let mut size = 0u32;
        let mut data: *mut c_void = std::ptr::null_mut();
        // SAFETY: wlanapi allocates `data`; it is freed below.
        unsafe {
            let status =
                WlanQueryInterface(self.handle, interface, wlan_intf_opcode_current_connection, None, &mut size, &mut data, None);
            match WlanError::from_code(status) {
                _ if status == ERROR_SUCCESS.0 => {}
                WlanError::AccessDenied => return Err(WlanError::AccessDenied),
                // ERROR_INVALID_STATE: not connected.
                _ => return Ok(None),
            }
            let attributes = &*(data as *const WLAN_CONNECTION_ATTRIBUTES);
            let association = &attributes.wlanAssociationAttributes;
            let connection = Connection {
                ssid: ssid_bytes(&association.dot11Ssid),
                profile: from_wide(&attributes.strProfileName),
                quality: association.wlanSignalQuality,
                auth: attributes.wlanSecurityAttributes.dot11AuthAlgorithm.0,
                bssid: association.dot11Bssid,
            };
            WlanFreeMemory(data);
            Ok(Some(connection))
        }
    }

    /// The networks Windows saw at the last scan.
    pub fn available(&self, interface: &GUID) -> Result<Vec<Available>, WlanError> {
        let mut list: *mut WLAN_AVAILABLE_NETWORK_LIST = std::ptr::null_mut();
        // SAFETY: the list is allocated by wlanapi and freed below.
        unsafe {
            check(WlanGetAvailableNetworkList(self.handle, interface, 0, None, &mut list))?;
            let items: &[WLAN_AVAILABLE_NETWORK] =
                std::slice::from_raw_parts((*list).Network.as_ptr(), (*list).dwNumberOfItems as usize);
            let out = items
                .iter()
                .filter(|n| n.dot11BssType == dot11_BSS_type_infrastructure)
                .map(|n| Available {
                    ssid: ssid_bytes(&n.dot11Ssid),
                    profile: from_wide(&n.strProfileName),
                    quality: n.wlanSignalQuality,
                    secure: n.bSecurityEnabled.as_bool(),
                    auth: n.dot11DefaultAuthAlgorithm.0,
                    connected: n.dwFlags & WLAN_AVAILABLE_NETWORK_CONNECTED != 0,
                    has_profile: n.dwFlags & WLAN_AVAILABLE_NETWORK_HAS_PROFILE != 0,
                })
                .collect();
            WlanFreeMemory(list.cast());
            Ok(out)
        }
    }

    /// Channel centre frequency (kHz) of the strongest access point per SSID,
    /// plus the frequency per BSSID.
    pub fn frequencies(&self, interface: &GUID) -> (HashMap<Vec<u8>, u32>, HashMap<[u8; 6], u32>) {
        let mut by_ssid: HashMap<Vec<u8>, (u32, u32)> = HashMap::new();
        let mut by_bssid = HashMap::new();
        let mut list: *mut WLAN_BSS_LIST = std::ptr::null_mut();
        // SAFETY: the list is allocated by wlanapi and freed below.
        unsafe {
            if check(WlanGetNetworkBssList(self.handle, interface, None, dot11_BSS_type_infrastructure, false, None, &mut list))
                .is_err()
            {
                return (HashMap::new(), HashMap::new());
            }
            let items: &[WLAN_BSS_ENTRY] =
                std::slice::from_raw_parts((*list).wlanBssEntries.as_ptr(), (*list).dwNumberOfItems as usize);
            for entry in items {
                by_bssid.insert(entry.dot11Bssid, entry.ulChCenterFrequency);
                let best = by_ssid.entry(ssid_bytes(&entry.dot11Ssid)).or_insert((0, 0));
                if entry.uLinkQuality >= best.0 {
                    *best = (entry.uLinkQuality, entry.ulChCenterFrequency);
                }
            }
            WlanFreeMemory(list.cast());
        }
        (by_ssid.into_iter().map(|(ssid, (_, frequency))| (ssid, frequency)).collect(), by_bssid)
    }

    /// Asks the driver to scan; completion arrives as [`WlanEvent::ScanComplete`].
    pub fn scan(&self, interface: &GUID) -> Result<(), WlanError> {
        // SAFETY: valid handle and interface.
        check(unsafe { WlanScan(self.handle, interface, None, None, None) })
    }

    /// Stores a profile (all users; per user if that is not allowed).
    pub fn set_profile(&self, interface: &GUID, xml: &str) -> Result<(), WlanError> {
        let xml = HSTRING::from(xml);
        let mut reason = 0u32;
        // SAFETY: NUL-terminated XML; out-pointer to a local.
        let status = unsafe { WlanSetProfile(self.handle, interface, 0, &xml, PCWSTR::null(), true, None, &mut reason) };
        if status == ERROR_ACCESS_DENIED.0 {
            // SAFETY: as above.
            return check(unsafe {
                WlanSetProfile(self.handle, interface, WLAN_PROFILE_USER, &xml, PCWSTR::null(), true, None, &mut reason)
            });
        }
        check(status)
    }

    /// Removes a profile.
    pub fn delete_profile(&self, interface: &GUID, name: &str) {
        // SAFETY: NUL-terminated profile name.
        let _ = unsafe { WlanDeleteProfile(self.handle, interface, &HSTRING::from(name), None) };
    }

    /// Connects with a stored profile; the result arrives as a notification.
    pub fn connect(&self, interface: &GUID, profile: &str) -> Result<(), WlanError> {
        let name = HSTRING::from(profile);
        let parameters = WLAN_CONNECTION_PARAMETERS {
            wlanConnectionMode: wlan_connection_mode_profile,
            strProfile: PCWSTR(name.as_ptr()),
            pDot11Ssid: std::ptr::null_mut(),
            pDesiredBssidList: std::ptr::null_mut(),
            dot11BssType: dot11_BSS_type_infrastructure,
            dwFlags: 0,
        };
        // SAFETY: the parameters reference `name`, which outlives the call.
        check(unsafe { WlanConnect(self.handle, interface, &parameters, None) })
    }

    /// Disconnects the interface.
    pub fn disconnect(&self, interface: &GUID) -> Result<(), WlanError> {
        // SAFETY: valid handle and interface.
        check(unsafe { WlanDisconnect(self.handle, interface, None) })
    }
}

impl Drop for Wlan {
    fn drop(&mut self) {
        // SAFETY: closes the handle opened in `open` (also ends the notifications).
        let _ = unsafe { WlanCloseHandle(self.handle, None) };
    }
}

fn ssid_bytes(ssid: &DOT11_SSID) -> Vec<u8> {
    let len = (ssid.uSSIDLength as usize).min(ssid.ucSSID.len());
    ssid.ucSSID[..len].to_vec()
}

unsafe extern "system" fn notification(data: *mut L2_NOTIFICATION_DATA, context: *mut c_void) {
    if data.is_null() || context.is_null() {
        return;
    }
    // SAFETY: wlanapi passes valid notification data; the context is the Arc
    // leaked in `Wlan::open`, alive for the process.
    let (data, context) = unsafe { (&*data, &*(context as *const Context)) };
    let code = data.NotificationCode as i32;
    let event = if data.NotificationSource == WLAN_NOTIFICATION_SOURCE_ACM {
        match code {
            c if c == wlan_notification_acm_scan_complete.0 => Some(WlanEvent::ScanComplete),
            c if c == wlan_notification_acm_scan_fail.0 => Some(WlanEvent::ScanFailed),
            c if c == wlan_notification_acm_connection_complete.0 || c == wlan_notification_acm_connection_attempt_fail.0 => {
                // SAFETY: these codes carry WLAN_CONNECTION_NOTIFICATION_DATA.
                let info = unsafe { connection_data(data) };
                info.map(|(profile, reason)| {
                    if c == wlan_notification_acm_connection_complete.0 && reason == 0 {
                        WlanEvent::ConnectionComplete { profile, reason }
                    } else {
                        WlanEvent::ConnectionFailed { profile, reason }
                    }
                })
            }
            c if c == wlan_notification_acm_disconnected.0
                || c == wlan_notification_acm_interface_arrival.0
                || c == wlan_notification_acm_interface_removal.0 =>
            {
                Some(WlanEvent::Changed)
            }
            _ => None,
        }
    } else if data.NotificationSource == WLAN_NOTIFICATION_SOURCE_MSM {
        let interesting = [wlan_notification_msm_connected, wlan_notification_msm_disconnected, wlan_notification_msm_radio_state_change];
        interesting.iter().any(|n| n.0 == code).then_some(WlanEvent::Changed)
    } else {
        None
    };
    let Some(event) = event else { return };
    if matches!(event, WlanEvent::Changed | WlanEvent::ConnectionComplete { .. } | WlanEvent::ConnectionFailed { .. }) {
        (context.on_change)();
    }
    let mut listeners = context.listeners.lock().unwrap_or_else(PoisonError::into_inner);
    listeners.retain(|tx| tx.send(event.clone()).is_ok());
}

/// # Safety
/// `data` must carry `WLAN_CONNECTION_NOTIFICATION_DATA`.
unsafe fn connection_data(data: &L2_NOTIFICATION_DATA) -> Option<(String, u32)> {
    if data.pData.is_null() || (data.dwDataSize as usize) < std::mem::size_of::<WLAN_CONNECTION_NOTIFICATION_DATA>() {
        return None;
    }
    // SAFETY: size checked above; guaranteed by the caller.
    let info = unsafe { &*(data.pData as *const WLAN_CONNECTION_NOTIFICATION_DATA) };
    Some((from_wide(&info.strProfileName), info.wlanReasonCode))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cargo test -p rice-win prints_wifi -- --ignored --nocapture` (read-only)
    #[test]
    #[ignore = "diagnostic output of the Wi-Fi state"]
    fn prints_wifi() {
        let wlan = Wlan::open(|| {}).expect("wlan");
        let interface = wlan.interface().expect("interface");
        match wlan.current(&interface) {
            Ok(Some(c)) => println!("connected: quality {} auth {}", c.quality, c.auth),
            other => println!("current: {other:?}"),
        }
        match wlan.available(&interface) {
            Ok(list) => println!("available: {} entries, {} with profile", list.len(), list.iter().filter(|n| n.has_profile).count()),
            Err(error) => println!("available: {error:?}"),
        }
        let (by_ssid, by_bssid) = wlan.frequencies(&interface);
        println!("frequencies: {} SSIDs, {} BSSIDs", by_ssid.len(), by_bssid.len());
    }
}
