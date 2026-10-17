//! Paired Bluetooth devices (classic and LE), their battery, change
//! notifications, and connecting audio devices.
//!
//! Connecting uses the kernel-streaming property Windows' own Settings use
//! for audio devices (`KSPROPERTY_ONESHOT_RECONNECT/DISCONNECT` on the
//! device's audio filter, the "ToothTray" approach). Other devices can only be
//! connected from Windows' Settings.

use crate::com::ComGuard;
use rice_core::network::BluetoothHints;
use std::collections::HashSet;
use std::sync::Arc;
use windows::Devices::Bluetooth::{BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationKind, DeviceWatcher};
use windows::Foundation::{IReference, TypedEventHandler};
use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::{
    DEVICE_STATE, DEVICE_STATEMASK_ALL, IConnector, IDeviceTopology, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eAll,
};
use windows::Win32::Media::KernelStreaming::{IKsControl, KSIDENTIFIER, KSIDENTIFIER_0, KSIDENTIFIER_0_0, KSPROPERTY_TYPE_GET};
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PropVariantToGUID};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::{GUID, HSTRING, IInspectable, Interface};
use windows_collections::{IIterable, IMapView};

const IS_CONNECTED: &str = "System.Devices.Aep.IsConnected";
const CONTAINER: &str = "System.Devices.Aep.ContainerId";
const COD_MAJOR: &str = "System.Devices.Aep.Bluetooth.Cod.Major";
const COD_MINOR: &str = "System.Devices.Aep.Bluetooth.Cod.Minor";
const LE_CATEGORY: &str = "System.Devices.Aep.Bluetooth.Le.Appearance.Category";
const LE_SUBCATEGORY: &str = "System.Devices.Aep.Bluetooth.Le.Appearance.Subcategory";
/// Battery level (percent) Windows shows under "Bluetooth & devices".
const BATTERY: &str = "{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2";
const DEVICE_CONTAINER: &str = "System.Devices.ContainerId";

/// `KSPROPSETID_BtAudio`
const BT_AUDIO: GUID = GUID::from_u128(0x7fa06c40_b8f6_4c7e_8556_e8c33a12e54d);
const ONESHOT_RECONNECT: u32 = 0;
const ONESHOT_DISCONNECT: u32 = 1;
/// `PKEY_Device_ContainerId`
const PKEY_CONTAINER: PROPERTYKEY =
    PROPERTYKEY { fmtid: GUID::from_u128(0x8c7ed206_3f8a_4827_b3ab_ae9e1faefc6c), pid: 2 };

/// A paired device as Windows reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct Paired {
    pub id: String,
    pub name: String,
    pub connected: bool,
    pub container: Option<GUID>,
    pub hints: BluetoothHints,
}

fn properties() -> IIterable<HSTRING> {
    IIterable::from(
        [IS_CONNECTED, CONTAINER, COD_MAJOR, COD_MINOR, LE_CATEGORY, LE_SUBCATEGORY].map(HSTRING::from).to_vec(),
    )
}

fn selectors() -> windows::core::Result<[HSTRING; 2]> {
    Ok([BluetoothDevice::GetDeviceSelectorFromPairingState(true)?, BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)?])
}

/// All paired devices, one entry per physical device (dual-mode devices are
/// paired twice, classic and LE, with the same container). Blocks.
pub fn paired() -> windows::core::Result<Vec<Paired>> {
    let mut out: Vec<Paired> = Vec::new();
    for selector in selectors()? {
        let found = DeviceInformation::FindAllAsyncAqsFilterAndAdditionalProperties(&selector, &properties())?.join()?;
        for i in 0..found.Size()? {
            let info = found.GetAt(i)?;
            let props = info.Properties()?;
            let device = Paired {
                id: info.Id()?.to_string(),
                name: info.Name()?.to_string(),
                connected: value::<bool>(&props, IS_CONNECTED).unwrap_or(false),
                container: value::<GUID>(&props, CONTAINER),
                hints: BluetoothHints {
                    cod_major: number(&props, COD_MAJOR),
                    cod_minor: number(&props, COD_MINOR),
                    le_category: number(&props, LE_CATEGORY),
                    le_subcategory: number(&props, LE_SUBCATEGORY),
                },
            };
            match out.iter_mut().find(|d| d.container.is_some() && d.container == device.container) {
                Some(existing) => {
                    existing.connected |= device.connected;
                    if existing.hints.cod_major.is_none() {
                        existing.hints.le_category = existing.hints.le_category.or(device.hints.le_category);
                        existing.hints.le_subcategory = existing.hints.le_subcategory.or(device.hints.le_subcategory);
                    }
                }
                None => out.push(device),
            }
        }
    }
    Ok(out)
}

fn value<T: windows::core::RuntimeType + 'static>(props: &IMapView<HSTRING, IInspectable>, key: &str) -> Option<T> {
    props.Lookup(&HSTRING::from(key)).ok()?.cast::<IReference<T>>().ok()?.Value().ok()
}

/// Integer properties come as u8, u16 or u32 depending on the property.
fn number(props: &IMapView<HSTRING, IInspectable>, key: &str) -> Option<u32> {
    value::<u32>(props, key)
        .or_else(|| value::<u16>(props, key).map(u32::from))
        .or_else(|| value::<u8>(props, key).map(u32::from))
}

/// Battery level in percent, read from the device nodes of a container. Blocks.
pub fn battery(container: GUID) -> Option<u8> {
    let filter = HSTRING::from(format!("{DEVICE_CONTAINER}:=\"{{{container:?}}}\""));
    let props = IIterable::from(vec![HSTRING::from(BATTERY)]);
    let nodes =
        DeviceInformation::FindAllAsyncWithKindAqsFilterAndAdditionalProperties(&filter, &props, DeviceInformationKind::Device)
            .ok()?
            .join()
            .ok()?;
    (0..nodes.Size().ok()?)
        .filter_map(|i| nodes.GetAt(i).ok())
        .find_map(|node| node.Properties().ok().and_then(|p| number(&p, BATTERY)))
        .and_then(|percent| u8::try_from(percent.min(100)).ok())
}

/// Keeps the device watchers alive.
pub struct Watchers(Vec<DeviceWatcher>);

impl Drop for Watchers {
    fn drop(&mut self) {
        for watcher in &self.0 {
            let _ = watcher.Stop();
        }
    }
}

/// Reports pairing, connection and property changes through `on_change`.
pub fn watch(on_change: impl Fn() + Send + Sync + 'static) -> windows::core::Result<Watchers> {
    let on_change = Arc::new(on_change);
    let mut watchers = Vec::new();
    for selector in selectors()? {
        let watcher = DeviceInformation::CreateWatcherAqsFilterAndAdditionalProperties(&selector, &properties())?;
        let notify = Arc::clone(&on_change);
        watcher.Added(&TypedEventHandler::new(move |_, _| {
            notify();
            Ok(())
        }))?;
        let notify = Arc::clone(&on_change);
        watcher.Updated(&TypedEventHandler::new(move |_, _| {
            notify();
            Ok(())
        }))?;
        let notify = Arc::clone(&on_change);
        watcher.Removed(&TypedEventHandler::new(move |_, _| {
            notify();
            Ok(())
        }))?;
        watcher.Start()?;
        watchers.push(watcher);
    }
    Ok(Watchers(watchers))
}

/// Containers of Bluetooth audio devices that can be (dis)connected from here.
/// Blocks; needs COM.
pub fn switchable_containers() -> HashSet<GUID> {
    let _com = ComGuard::mta();
    bluetooth_audio_filters().into_iter().map(|(container, _)| container).collect()
}

/// Connects or disconnects the audio device in `container`. Blocks.
pub fn switch_audio(container: GUID, connect: bool) -> bool {
    let _com = ComGuard::mta();
    let id = if connect { ONESHOT_RECONNECT } else { ONESHOT_DISCONNECT };
    let mut switched = false;
    for (_, control) in bluetooth_audio_filters().into_iter().filter(|(c, _)| *c == container) {
        let property = KSIDENTIFIER {
            Anonymous: KSIDENTIFIER_0 { Anonymous: KSIDENTIFIER_0_0 { Set: BT_AUDIO, Id: id, Flags: KSPROPERTY_TYPE_GET } },
        };
        let mut returned = 0u32;
        // SAFETY: a property request without data; the filter acts on the request itself.
        let result = unsafe {
            control.KsProperty(&property, std::mem::size_of::<KSIDENTIFIER>() as u32, std::ptr::null_mut(), 0, &mut returned)
        };
        match result {
            Ok(()) => switched = true,
            Err(error) => log::debug!("bluetooth: audio filter refused ({error})"),
        }
    }
    switched
}

/// (container, kernel-streaming control) of every Bluetooth audio filter,
/// connected or not; one device may have several (stereo and hands-free).
fn bluetooth_audio_filters() -> Vec<(GUID, IKsControl)> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    // SAFETY: COM calls on this thread; strings and PROPVARIANTs are released.
    unsafe {
        let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else {
            return out;
        };
        let Ok(collection) = enumerator.EnumAudioEndpoints(eAll, DEVICE_STATE(DEVICE_STATEMASK_ALL)) else {
            return out;
        };
        for i in 0..collection.GetCount().unwrap_or(0) {
            let Ok(device) = collection.Item(i) else { continue };
            let Some(container) = container_of(&device) else { continue };
            let Some(filter) = bluetooth_filter_id(&device) else { continue };
            if !seen.insert(filter.clone()) {
                continue;
            }
            let control = enumerator
                .GetDevice(&HSTRING::from(filter.as_str()))
                .and_then(|f| f.Activate::<IKsControl>(CLSCTX_ALL, None));
            match control {
                Ok(control) => out.push((container, control)),
                Err(error) => log::debug!("bluetooth: no kernel-streaming control for {filter}: {error}"),
            }
        }
    }
    out
}

/// # Safety
/// COM must be initialised.
unsafe fn container_of(device: &IMMDevice) -> Option<GUID> {
    // SAFETY: guaranteed by the caller; the PROPVARIANT is cleared.
    unsafe {
        let store = device.OpenPropertyStore(STGM_READ).ok()?;
        let mut value = store.GetValue(&PKEY_CONTAINER).ok()?;
        let guid = PropVariantToGUID(&value).ok();
        let _ = PropVariantClear(&mut value);
        guid
    }
}

/// Device id of the driver filter behind an endpoint, if it is the device's
/// own Bluetooth audio driver. Shared offload filters (e.g. Qualcomm `aucd`
/// on Snapdragon machines) serve every device and are skipped: a reconnect
/// request there would be ambiguous.
///
/// # Safety
/// COM must be initialised.
unsafe fn bluetooth_filter_id(device: &IMMDevice) -> Option<String> {
    // SAFETY: guaranteed by the caller; the id string is freed.
    unsafe {
        let topology: IDeviceTopology = device.Activate(CLSCTX_ALL, None).ok()?;
        let connector: IConnector = topology.GetConnector(0).ok()?;
        let raw = connector.GetDeviceIdConnectedTo().ok()?;
        let id = raw.to_string().unwrap_or_default();
        CoTaskMemFree(Some(raw.0 as *const _));
        let lower = id.to_ascii_lowercase();
        (lower.contains("bthenum") || lower.contains("bthhfenum")).then_some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::radios::{Radios, hotspot, internet_connection};

    /// `cargo test -p rice-win prints_connectivity -- --ignored --nocapture` (read-only)
    #[test]
    #[ignore = "diagnostic output of radios and paired devices"]
    fn prints_connectivity() {
        let radios = Radios::load(|| {}).expect("radios");
        println!("radios: {:?}, all off: {}", radios.states(), radios.all_off());
        println!("hotspot: {:?}", hotspot());
        println!("internet: {:?}", internet_connection());
        for device in paired().expect("paired devices") {
            println!("{device:?} battery {:?}", device.container.and_then(battery));
        }
        println!("switchable audio: {:?}", switchable_containers());
    }
}
