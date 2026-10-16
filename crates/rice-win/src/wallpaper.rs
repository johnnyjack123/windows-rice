//! Desktop wallpaper of one monitor via `IDesktopWallpaper`.

use crate::com::ComGuard;
use rice_core::contract::WallpaperFit;
use rice_core::geometry::PhysRect;
use std::path::PathBuf;
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree};
use windows::Win32::UI::Shell::{
    DESKTOP_WALLPAPER_POSITION, DWPOS_CENTER, DWPOS_FILL, DWPOS_FIT, DWPOS_SPAN, DWPOS_STRETCH, DWPOS_TILE, DesktopWallpaper,
    IDesktopWallpaper,
};
use windows::core::{PCWSTR, PWSTR};

/// Where the wallpaper of a monitor comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WallpaperSource {
    /// Image file; `None` for a solid colour.
    pub path: Option<PathBuf>,
    /// Background colour as COLORREF (`0x00BBGGRR`).
    pub color: u32,
    pub fit: WallpaperFit,
}

/// Queries the wallpaper of the monitor occupying `monitor` (physical rect).
pub fn query(monitor: PhysRect) -> windows::core::Result<WallpaperSource> {
    let _com = ComGuard::mta();
    // SAFETY: COM calls on an interface we own; every returned string is freed.
    unsafe {
        let desktop: IDesktopWallpaper = CoCreateInstance(&DesktopWallpaper, None, CLSCTX_ALL)?;
        let id = find_monitor(&desktop, monitor);
        let path = match &id {
            Some(id) => desktop.GetWallpaper(PCWSTR(id.as_ptr())),
            // Not found (e.g. a mirrored display): all monitors share one image or none.
            None => desktop.GetWallpaper(PCWSTR::null()),
        }
        .map(|raw| take_string(raw))
        .unwrap_or_default();
        let fit = to_fit(desktop.GetPosition().unwrap_or(DWPOS_FILL));
        let color = desktop.GetBackgroundColor().map(|c| c.0).unwrap_or(0);
        Ok(WallpaperSource { path: (!path.is_empty()).then(|| PathBuf::from(path)), color, fit })
    }
}

/// Monitor id (NUL-terminated UTF-16) of the monitor with this rectangle.
///
/// # Safety
/// `desktop` must be a live `IDesktopWallpaper`.
unsafe fn find_monitor(desktop: &IDesktopWallpaper, monitor: PhysRect) -> Option<Vec<u16>> {
    // SAFETY: guaranteed by the caller; strings are copied and freed.
    unsafe {
        let count = desktop.GetMonitorDevicePathCount().ok()?;
        for i in 0..count {
            let Ok(raw) = desktop.GetMonitorDevicePathAt(i) else { continue };
            let rect = desktop.GetMonitorRECT(PCWSTR(raw.0));
            let id: Vec<u16> = take_string(raw).encode_utf16().chain(Some(0)).collect();
            if let Ok(r) = rect
                && PhysRect::from_edges(r.left, r.top, r.right, r.bottom) == monitor
            {
                return Some(id);
            }
        }
        None
    }
}

/// Copies a COM-allocated string and frees it.
///
/// # Safety
/// `text` must be null or allocated with `CoTaskMemAlloc`.
unsafe fn take_string(text: PWSTR) -> String {
    if text.is_null() {
        return String::new();
    }
    // SAFETY: guaranteed by the caller.
    unsafe {
        let value = text.to_string().unwrap_or_default();
        CoTaskMemFree(Some(text.0 as *const _));
        value
    }
}

fn to_fit(position: DESKTOP_WALLPAPER_POSITION) -> WallpaperFit {
    match position {
        DWPOS_FIT => WallpaperFit::Fit,
        DWPOS_STRETCH => WallpaperFit::Stretch,
        DWPOS_TILE => WallpaperFit::Tile,
        DWPOS_CENTER => WallpaperFit::Center,
        DWPOS_SPAN => WallpaperFit::Span,
        _ => WallpaperFit::Fill,
    }
}

/// Windows' own JPEG copy of the current wallpaper; used when the original
/// file is gone or in a format the decoder does not know (HEIC, …).
pub fn transcoded_fallback() -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("APPDATA")?).join("Microsoft\\Windows\\Themes\\TranscodedWallpaper");
    path.is_file().then_some(path)
}
