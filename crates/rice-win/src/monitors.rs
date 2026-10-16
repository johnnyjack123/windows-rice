//! Connected monitors with work area, DPI and a readable name.

use crate::from_wide;
use rice_core::geometry::PhysRect;
use std::collections::HashMap;
use std::mem;
use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO,
    DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo,
    GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::{ERROR_SUCCESS, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    DISPLAY_DEVICEW, EnumDisplayDevicesW, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;
use windows::core::{BOOL, PCWSTR};

/// One monitor as Windows reports it (physical pixels, virtual screen).
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    /// Position in `EnumDisplayMonitors` order (`settings.island.monitor`).
    pub index: usize,
    /// `HMONITOR` value; valid until the display configuration changes.
    pub handle: isize,
    /// GDI device name, e.g. `\\.\DISPLAY1`.
    pub device: String,
    /// Friendly name, e.g. "LG ULTRAGEAR".
    pub name: String,
    pub primary: bool,
    pub rect: PhysRect,
    pub work: PhysRect,
    /// DPI / 96.
    pub scale: f64,
}

/// All monitors in `EnumDisplayMonitors` order.
pub fn list() -> Vec<Monitor> {
    let mut handles: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback only pushes into the Vec passed through LPARAM,
    // which outlives the synchronous enumeration.
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(collect), LPARAM(&mut handles as *mut Vec<HMONITOR> as isize));
    }
    let names = friendly_names();
    handles
        .into_iter()
        .filter_map(describe)
        .enumerate()
        .map(|(index, mut monitor)| {
            monitor.index = index;
            if let Some(name) = names.get(&monitor.device).filter(|n| !n.is_empty()) {
                monitor.name = name.clone();
            }
            monitor
        })
        .collect()
}

unsafe extern "system" fn collect(monitor: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
    // SAFETY: `data` is the &mut Vec handed to EnumDisplayMonitors above.
    let handles = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
    handles.push(monitor);
    BOOL(1)
}

fn describe(handle: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: MONITORINFOEXW starts with MONITORINFO and cbSize says which one it is.
    let ok = unsafe { GetMonitorInfoW(handle, &mut info as *mut MONITORINFOEXW as *mut MONITORINFO) };
    if !ok.as_bool() {
        return None;
    }
    let (mut dpi_x, mut dpi_y) = (96u32, 96u32);
    // SAFETY: out-pointers to locals.
    if unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }.is_err() {
        dpi_x = 96;
    }
    let device = from_wide(&info.szDevice);
    let r = info.monitorInfo.rcMonitor;
    let w = info.monitorInfo.rcWork;
    Some(Monitor {
        index: 0,
        handle: handle.0 as isize,
        name: adapter_monitor_name(&device).unwrap_or_else(|| device.trim_start_matches("\\\\.\\").to_owned()),
        device,
        primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0,
        rect: PhysRect::from_edges(r.left, r.top, r.right, r.bottom),
        work: PhysRect::from_edges(w.left, w.top, w.right, w.bottom),
        scale: f64::from(dpi_x.max(1)) / 96.0,
    })
}

/// Fallback name from the display adapter's monitor entry ("Generic PnP Monitor").
fn adapter_monitor_name(device: &str) -> Option<String> {
    let wide: Vec<u16> = device.encode_utf16().chain(Some(0)).collect();
    let mut entry = DISPLAY_DEVICEW { cb: mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
    // SAFETY: NUL-terminated device name and a correctly sized out struct.
    let ok = unsafe { EnumDisplayDevicesW(PCWSTR(wide.as_ptr()), 0, &mut entry, 0) };
    ok.as_bool().then(|| from_wide(&entry.DeviceString)).filter(|n| !n.is_empty())
}

/// Maps GDI device names to EDID friendly names via the display configuration API.
fn friendly_names() -> HashMap<String, String> {
    let mut names = HashMap::new();
    let (mut path_count, mut mode_count) = (0u32, 0u32);
    // SAFETY: out-pointers to locals.
    if unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count) } != ERROR_SUCCESS {
        return names;
    }
    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
    // SAFETY: buffers sized by GetDisplayConfigBufferSizes; counts are updated in place.
    let status = unsafe {
        QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut path_count, paths.as_mut_ptr(), &mut mode_count, modes.as_mut_ptr(), None)
    };
    if status != ERROR_SUCCESS {
        return names;
    }
    paths.truncate(path_count as usize);
    for path in paths {
        let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
        source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
        source.header.size = mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
        source.header.adapterId = path.sourceInfo.adapterId;
        source.header.id = path.sourceInfo.id;
        let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
        target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
        target.header.size = mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
        target.header.adapterId = path.targetInfo.adapterId;
        target.header.id = path.targetInfo.id;
        // SAFETY: each packet starts with its header and declares its own size.
        let ok = unsafe { DisplayConfigGetDeviceInfo(&mut source.header) == 0 && DisplayConfigGetDeviceInfo(&mut target.header) == 0 };
        if ok {
            names.insert(from_wide(&source.viewGdiDeviceName), from_wide(&target.monitorFriendlyDeviceName));
        }
    }
    names
}
