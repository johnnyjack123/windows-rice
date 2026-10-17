//! Radios (Wi-Fi, Bluetooth, mobile broadband) via `Windows.Devices.Radios`,
//! the mobile hotspot and network status notifications.

use std::sync::Arc;
use windows::Devices::Radios::{Radio, RadioAccessStatus, RadioKind, RadioState};
use windows::Foundation::TypedEventHandler;
use windows::Networking::Connectivity::{NetworkInformation, NetworkStatusChangedEventHandler};
use windows::Networking::NetworkOperators::{
    NetworkOperatorTetheringManager, TetheringCapability, TetheringOperationalState, TetheringOperationStatus,
};

/// Which radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    WiFi,
    Bluetooth,
    MobileBroadband,
}

impl Kind {
    fn matches(self, kind: RadioKind) -> bool {
        match self {
            Self::WiFi => kind == RadioKind::WiFi,
            Self::Bluetooth => kind == RadioKind::Bluetooth,
            Self::MobileBroadband => kind == RadioKind::MobileBroadband,
        }
    }
}

/// The machine's radios with change notifications.
pub struct Radios {
    radios: Vec<(Kind, Radio)>,
}

impl Radios {
    /// Loads the radios and reports state changes through `on_change`.
    /// Blocks (WinRT `join`): call from a worker thread.
    pub fn load(on_change: impl Fn() + Send + Sync + 'static) -> windows::core::Result<Self> {
        let access = Radio::RequestAccessAsync()?.join()?;
        if access != RadioAccessStatus::Allowed {
            log::warn!("radios: access {access:?}; switching radios may fail");
        }
        let on_change = Arc::new(on_change);
        let list = Radio::GetRadiosAsync()?.join()?;
        let mut radios = Vec::new();
        for i in 0..list.Size()? {
            let radio = list.GetAt(i)?;
            let Ok(kind) = radio.Kind() else { continue };
            let Some(kind) = [Kind::WiFi, Kind::Bluetooth, Kind::MobileBroadband].into_iter().find(|k| k.matches(kind)) else {
                continue;
            };
            let notify = Arc::clone(&on_change);
            radio.StateChanged(&TypedEventHandler::new(move |_, _| {
                notify();
                Ok(())
            }))?;
            radios.push((kind, radio));
        }
        Ok(Self { radios })
    }

    /// Whether a radio of this kind exists.
    pub fn has(&self, kind: Kind) -> bool {
        self.radios.iter().any(|(k, _)| *k == kind)
    }

    /// Whether a radio of this kind is on.
    pub fn is_on(&self, kind: Kind) -> bool {
        self.radios.iter().any(|(k, r)| *k == kind && r.State().is_ok_and(|s| s == RadioState::On))
    }

    /// Current on/off state of every radio.
    pub fn states(&self) -> Vec<(Kind, bool)> {
        self.radios.iter().map(|(kind, radio)| (*kind, radio.State().is_ok_and(|s| s == RadioState::On))).collect()
    }

    /// Airplane mode as the panel sees it: radios exist and all are off.
    pub fn all_off(&self) -> bool {
        !self.radios.is_empty() && self.radios.iter().all(|(_, r)| r.State().is_ok_and(|s| s != RadioState::On))
    }

    /// Switches every radio of one kind. Blocks.
    pub fn set(&self, kind: Kind, on: bool) -> windows::core::Result<bool> {
        let state = if on { RadioState::On } else { RadioState::Off };
        let mut allowed = false;
        for (_, radio) in self.radios.iter().filter(|(k, _)| *k == kind) {
            allowed |= radio.SetStateAsync(state)?.join()? == RadioAccessStatus::Allowed;
        }
        Ok(allowed)
    }
}

/// Calls `on_change` whenever Windows' network status changes (connected,
/// disconnected, metered …). Returns the registration token.
pub fn watch_network_status(on_change: impl Fn() + Send + Sync + 'static) -> windows::core::Result<i64> {
    NetworkInformation::NetworkStatusChanged(&NetworkStatusChangedEventHandler::new(move |_| {
        on_change();
        Ok(())
    }))
}

/// The internet connection's profile name and signal bars (0…5); readable
/// without location access, unlike the WLAN details.
pub fn internet_connection() -> Option<(String, Option<u8>)> {
    let profile = NetworkInformation::GetInternetConnectionProfile().ok()?;
    if !profile.IsWlanConnectionProfile().unwrap_or(false) {
        return None;
    }
    let name = profile.ProfileName().ok()?.to_string();
    let bars = profile.GetSignalBars().ok().and_then(|b| b.Value().ok());
    Some((name, bars))
}

/// Mobile hotspot state: `None` if it cannot be controlled here.
pub fn hotspot() -> Option<bool> {
    let manager = hotspot_manager()?;
    Some(manager.TetheringOperationalState().ok()? == TetheringOperationalState::On)
}

/// Switches the mobile hotspot. Blocks.
pub fn set_hotspot(on: bool) -> windows::core::Result<bool> {
    let Some(manager) = hotspot_manager() else {
        return Ok(false);
    };
    let result = if on { manager.StartTetheringAsync()?.join()? } else { manager.StopTetheringAsync()?.join()? };
    Ok(result.Status()? == TetheringOperationStatus::Success)
}

fn hotspot_manager() -> Option<NetworkOperatorTetheringManager> {
    let profile = NetworkInformation::GetInternetConnectionProfile().ok()?;
    let capability = NetworkOperatorTetheringManager::GetTetheringCapabilityFromConnectionProfile(&profile).ok()?;
    if capability != TetheringCapability::Enabled {
        return None;
    }
    NetworkOperatorTetheringManager::CreateFromConnectionProfile(&profile).ok()
}
