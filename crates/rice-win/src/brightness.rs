//! Display brightness: DDC/CI for external monitors (dxva2), WMI for the
//! built-in panel of laptops (`root\WMI`, `WmiMonitorBrightness`).
//!
//! DDC/CI calls take tens of milliseconds and WMI blocks: call from workers.

use crate::com::ComGuard;
use rice_core::devices::{ddc_fraction, ddc_value, percent};
use std::thread;
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitors, GetMonitorBrightness, GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR,
    PHYSICAL_MONITOR, SetMonitorBrightness,
};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CoSetProxyBlanket, EOAC_NONE, RPC_C_AUTHN_LEVEL_CALL, RPC_C_IMP_LEVEL_IMPERSONATE,
};
use windows::Win32::System::Rpc::{RPC_C_AUTHN_WINNT, RPC_C_AUTHZ_NONE};
use windows::Win32::System::Variant::{VARIANT, VT_I4, VariantClear, VariantToStringAlloc, VariantToUInt32};
use windows::Win32::System::Wmi::{
    IEnumWbemClassObject, IWbemClassObject, IWbemLocator, IWbemServices, WBEM_FLAG_FORWARD_ONLY, WBEM_FLAG_RETURN_IMMEDIATELY,
    WBEM_FLAG_RETURN_WBEM_COMPLETE, WBEM_INFINITE, WbemLocator,
};
use windows::core::{BSTR, HSTRING, IUnknown, PCWSTR, Param};

// ── DDC/CI (external monitors) ───────────────────────────────────────────

/// Brightness of a monitor over DDC/CI, if it supports it.
pub fn ddc_get(monitor: isize) -> Option<f64> {
    with_physical_monitor(monitor, |handle| {
        let (mut min, mut current, mut max) = (0u32, 0u32, 0u32);
        // SAFETY: valid physical monitor handle and out-pointers to locals.
        let ok = unsafe { GetMonitorBrightness(handle, &mut min, &mut current, &mut max) } != 0;
        if ok { ddc_fraction(min, current, max) } else { None }
    })
}

/// Sets the brightness of a monitor over DDC/CI.
pub fn ddc_set(monitor: isize, fraction: f64) -> bool {
    with_physical_monitor(monitor, |handle| {
        let (mut min, mut current, mut max) = (0u32, 0u32, 0u32);
        // SAFETY: valid physical monitor handle and out-pointers to locals.
        unsafe {
            if GetMonitorBrightness(handle, &mut min, &mut current, &mut max) == 0 {
                return None;
            }
            (SetMonitorBrightness(handle, ddc_value(fraction, min, max)) != 0).then_some(())
        }
    })
    .is_some()
}

/// Runs `f` on the first physical monitor behind an `HMONITOR` that answers.
fn with_physical_monitor<R>(monitor: isize, mut f: impl FnMut(HANDLE) -> Option<R>) -> Option<R> {
    let hmonitor = HMONITOR(monitor as *mut _);
    let mut count = 0u32;
    // SAFETY: out-pointer to a local; the array is sized by the count and destroyed below.
    unsafe {
        GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count).ok()?;
        if count == 0 {
            return None;
        }
        let mut monitors = vec![PHYSICAL_MONITOR::default(); count as usize];
        GetPhysicalMonitorsFromHMONITOR(hmonitor, &mut monitors).ok()?;
        let result = monitors.iter().find_map(|m| f(m.hPhysicalMonitor));
        let _ = DestroyPhysicalMonitors(&monitors);
        result
    }
}

// ── WMI (built-in panel) ─────────────────────────────────────────────────

struct Wmi {
    services: IWbemServices,
}

impl Wmi {
    fn connect() -> windows::core::Result<Self> {
        // SAFETY: COM calls on a thread with COM initialised by the caller.
        unsafe {
            let locator: IWbemLocator = CoCreateInstance(&WbemLocator, None, CLSCTX_INPROC_SERVER)?;
            let empty = BSTR::new();
            let services = locator.ConnectServer(&BSTR::from("ROOT\\WMI"), &empty, &empty, &empty, 0, &empty, None)?;
            blanket(&services)?;
            Ok(Self { services })
        }
    }

    fn query(&self, wql: &str) -> windows::core::Result<Vec<IWbemClassObject>> {
        // SAFETY: forward-only enumeration of a query result.
        unsafe {
            let rows = self.services.ExecQuery(
                &BSTR::from("WQL"),
                &BSTR::from(wql),
                WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                None,
            )?;
            blanket(&rows)?;
            Ok(drain(&rows))
        }
    }
}

/// WMI needs impersonation rights on every proxy (set per interface pointer).
fn blanket<P: Param<IUnknown>>(proxy: P) -> windows::core::Result<()> {
    // SAFETY: standard security settings for local WMI proxies.
    unsafe {
        CoSetProxyBlanket(
            proxy,
            RPC_C_AUTHN_WINNT,
            RPC_C_AUTHZ_NONE,
            PCWSTR::null(),
            RPC_C_AUTHN_LEVEL_CALL,
            RPC_C_IMP_LEVEL_IMPERSONATE,
            None,
            EOAC_NONE,
        )
    }
}

/// # Safety
/// `rows` must be a live enumerator.
unsafe fn drain(rows: &IEnumWbemClassObject) -> Vec<IWbemClassObject> {
    let mut out = Vec::new();
    loop {
        let mut row = [None];
        let mut returned = 0u32;
        // SAFETY: one-element output array and an out-pointer to a local.
        let hr = unsafe { rows.Next(WBEM_INFINITE, &mut row, &mut returned) };
        if hr.is_err() || returned == 0 {
            break;
        }
        if let Some(object) = row[0].take() {
            out.push(object);
        }
    }
    out
}

fn get_u32(object: &IWbemClassObject, name: &str) -> Option<u32> {
    let name = HSTRING::from(name);
    let mut value = VARIANT::default();
    // SAFETY: the VARIANT is cleared after conversion.
    unsafe {
        object.Get(&name, 0, &mut value, None, None).ok()?;
        let number = VariantToUInt32(&value).ok();
        let _ = VariantClear(&mut value);
        number
    }
}

fn get_string(object: &IWbemClassObject, name: &str) -> Option<String> {
    let name = HSTRING::from(name);
    let mut value = VARIANT::default();
    // SAFETY: the VARIANT and the converted string are both released.
    unsafe {
        object.Get(&name, 0, &mut value, None, None).ok()?;
        let text = VariantToStringAlloc(&value).ok().map(|raw| {
            let text = raw.to_string().unwrap_or_default();
            windows::Win32::System::Com::CoTaskMemFree(Some(raw.0 as *const _));
            text
        });
        let _ = VariantClear(&mut value);
        text
    }
}

fn variant_i32(number: i32) -> VARIANT {
    let mut value = VARIANT::default();
    // SAFETY: writes the discriminant and the matching union member of a fresh VARIANT.
    unsafe {
        let inner = &mut *value.Anonymous.Anonymous;
        inner.vt = VT_I4;
        inner.Anonymous.lVal = number;
    }
    value
}

/// Brightness of the built-in panel via WMI, if the machine has one.
pub fn wmi_get() -> Option<f64> {
    let _com = ComGuard::mta();
    let wmi = Wmi::connect().ok()?;
    let rows = wmi.query("SELECT CurrentBrightness FROM WmiMonitorBrightness WHERE Active = TRUE").ok()?;
    let value = get_u32(rows.first()?, "CurrentBrightness")?;
    Some(f64::from(value.min(100)) / 100.0)
}

/// Sets the brightness of the built-in panel via WMI.
pub fn wmi_set(fraction: f64) -> windows::core::Result<()> {
    let _com = ComGuard::mta();
    let wmi = Wmi::connect()?;
    let missing = || windows::core::Error::from_hresult(windows::Win32::Foundation::E_FAIL);
    let rows = wmi.query("SELECT __PATH FROM WmiMonitorBrightnessMethods WHERE Active = TRUE")?;
    let path = rows.first().and_then(|row| get_string(row, "__PATH")).ok_or_else(missing)?;
    // SAFETY: WMI method call with an input object spawned from the method signature.
    unsafe {
        let mut class = None;
        wmi.services.GetObject(&BSTR::from("WmiMonitorBrightnessMethods"), WBEM_FLAG_RETURN_WBEM_COMPLETE, None, Some(&mut class), None)?;
        let class = class.ok_or_else(missing)?;
        let mut signature = None;
        class.GetMethod(&HSTRING::from("WmiSetBrightness"), 0, &mut signature, std::ptr::null_mut())?;
        let params = signature.ok_or_else(missing)?.SpawnInstance(0)?;
        params.Put(&HSTRING::from("Timeout"), 0, &variant_i32(0), 0)?;
        params.Put(&HSTRING::from("Brightness"), 0, &variant_i32(i32::from(percent(fraction))), 0)?;
        wmi.services.ExecMethod(
            &BSTR::from(path),
            &BSTR::from("WmiSetBrightness"),
            WBEM_FLAG_RETURN_WBEM_COMPLETE,
            None,
            &params,
            None,
            None,
        )
    }
}

/// Watches brightness changes of the built-in panel (brightness keys,
/// Windows' slider, adaptive brightness) on a dedicated thread.
pub fn watch_wmi(on_change: impl Fn(f64) + Send + 'static) -> std::io::Result<()> {
    thread::Builder::new().name("rice-brightness".into()).spawn(move || {
        let _com = ComGuard::mta();
        let events = Wmi::connect().and_then(|wmi| {
            // SAFETY: semi-synchronous event query; the enumerator blocks in Next.
            unsafe {
                let events = wmi.services.ExecNotificationQuery(
                    &BSTR::from("WQL"),
                    &BSTR::from("SELECT * FROM WmiMonitorBrightnessEvent"),
                    WBEM_FLAG_FORWARD_ONLY | WBEM_FLAG_RETURN_IMMEDIATELY,
                    None,
                )?;
                blanket(&events)?;
                Ok((wmi, events))
            }
        });
        let (_wmi, events) = match events {
            Ok(pair) => pair,
            Err(error) => {
                log::warn!("brightness: no WMI events ({error})");
                return;
            }
        };
        loop {
            let mut row = [None];
            let mut returned = 0u32;
            // SAFETY: blocks until the next event; one-element output array.
            let hr = unsafe { events.Next(WBEM_INFINITE, &mut row, &mut returned) };
            if hr.is_err() {
                log::warn!("brightness: WMI event stream ended ({hr:?})");
                return;
            }
            if let Some(value) = row[0].take().and_then(|event| get_u32(&event, "Brightness")) {
                on_change(f64::from(value.min(100)) / 100.0);
            }
        }
    })?;
    Ok(())
}
