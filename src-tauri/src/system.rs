//! Appearance, wallpaper, monitors and fonts, plus the reaction to system
//! notifications (settings, displays, window movement).

use crate::breaks;
use crate::display;
use crate::island;
use crate::power;
use crate::state::{AppState, Locked};
use rice_core::contract::{Appearance, MonitorInfo, WallpaperFit, WallpaperInfo, events};
use rice_core::geometry::{self, PhysRect};
use rice_core::{BackendError, BackendResult};
use rice_services::imaging::{self, Scaling};
use rice_win::events::{SystemChange, SystemEvent};
use rice_win::wallpaper::WallpaperSource;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;
use tauri::{AppHandle, Emitter, Manager};

/// Last values sent to the frontend and the encoded wallpaper.
#[derive(Default)]
pub struct SystemState {
    appearance: Mutex<Option<Appearance>>,
    wallpaper_sent: Mutex<Option<WallpaperInfo>>,
    /// Encoding a wallpaper takes a moment; concurrent requests wait for one result.
    wallpaper_cache: tokio::sync::Mutex<Option<(WallpaperKey, Option<String>)>>,
}

/// Identifies an encoded wallpaper: same file, same version, same target size.
#[derive(Debug, Clone, PartialEq)]
struct WallpaperKey {
    path: PathBuf,
    modified: Option<SystemTime>,
    len: u64,
    max: (u32, u32),
    scaling: Scaling,
}

/// Starts listening to system notifications and records the current appearance.
pub fn start(app: &AppHandle) -> std::io::Result<()> {
    *app.state::<AppState>().system.appearance.locked() = Some(rice_win::appearance::read());
    let handle = app.clone();
    rice_win::events::spawn(move |event| match event {
        SystemEvent::WindowsMoved => {
            island::refresh_occlusion(&handle);
        }
        SystemEvent::Changed(change) => on_change(&handle, change),
    })
}

fn on_change(app: &AppHandle, change: SystemChange) {
    if change.appearance {
        refresh_appearance(app);
    }
    if change.displays || change.work_area {
        // Sends the wallpaper itself when the window moved.
        island::reposition(app);
    }
    if change.wallpaper {
        emit_wallpaper(app);
    }
    if change.displays {
        display::reset(app);
        // Creating windows waits for the UI thread; this thread must keep pumping messages.
        let handle = app.clone();
        tauri::async_runtime::spawn_blocking(move || breaks::sync_monitors(&handle));
    }
    if change.power {
        power::refresh(app);
    }
}

/// `system_appearance`.
pub fn appearance() -> Appearance {
    rice_win::appearance::read()
}

fn refresh_appearance(app: &AppHandle) {
    let current = rice_win::appearance::read();
    let state = app.state::<AppState>();
    let changed = state.system.appearance.locked().replace(current.clone()).as_ref() != Some(&current);
    if changed {
        let _ = app.emit(events::APPEARANCE, current);
    }
}

/// `system_monitors`.
pub fn monitors() -> Vec<MonitorInfo> {
    rice_win::monitors::list()
        .into_iter()
        .map(|m| MonitorInfo {
            index: u32::try_from(m.index).unwrap_or(u32::MAX),
            name: m.name,
            primary: m.primary,
            width: u32::try_from(m.rect.width).unwrap_or(0),
            height: u32::try_from(m.rect.height).unwrap_or(0),
        })
        .collect()
}

/// `system_fonts`.
pub async fn fonts() -> BackendResult<Vec<String>> {
    tauri::async_runtime::spawn_blocking(rice_win::fonts::families)
        .await
        .map_err(BackendError::internal)?
        .map_err(BackendError::internal)
}

/// `system_wallpaper`: the island monitor's wallpaper and the window's position on it.
pub async fn wallpaper(app: &AppHandle) -> BackendResult<WallpaperInfo> {
    let state = app.state::<AppState>();
    let (monitor, window, scale) = match state.island.placement() {
        Some(p) => (p.monitor, p.window, p.scale),
        None => {
            let monitors = rice_win::monitors::list();
            let m = island::pick_monitor(&monitors, state.settings.locked().monitor)
                .ok_or_else(|| BackendError::not_found("no monitor"))?;
            (m.rect, m.rect, m.scale)
        }
    };
    let source = tauri::async_runtime::spawn_blocking(move || rice_win::wallpaper::query(monitor))
        .await
        .map_err(BackendError::internal)?
        .unwrap_or_else(|error| {
            log::warn!("wallpaper: IDesktopWallpaper failed ({error}); using Windows' copy");
            WallpaperSource { path: rice_win::wallpaper::transcoded_fallback(), color: 0, fit: WallpaperFit::Fill }
        });
    let url = encoded(&state, source.path.as_deref(), monitor, source.fit).await;
    let (monitor_size, window_point) = geometry::wallpaper_geometry(monitor, window, scale);
    Ok(WallpaperInfo {
        url,
        color: rice_win::colorref_to_hex(source.color),
        fit: source.fit,
        monitor: monitor_size,
        window: window_point,
    })
}

/// Sends `system://wallpaper` in the background, but only if something changed.
pub fn emit_wallpaper(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        match wallpaper(&handle).await {
            Ok(info) => {
                let state = handle.state::<AppState>();
                let changed = state.system.wallpaper_sent.locked().replace(info.clone()).as_ref() != Some(&info);
                if changed {
                    let _ = handle.emit(events::WALLPAPER, info);
                }
            }
            Err(error) => log::warn!("wallpaper: {error}"),
        }
    });
}

/// Data URL of the wallpaper image, from cache when file and size are unchanged.
async fn encoded(state: &AppState, path: Option<&Path>, monitor: PhysRect, fit: WallpaperFit) -> Option<String> {
    let path = path.filter(|p| p.is_file()).map(Path::to_path_buf).or_else(rice_win::wallpaper::transcoded_fallback)?;
    let scaling = match fit {
        WallpaperFit::Center | WallpaperFit::Tile => Scaling::Native,
        _ => Scaling::Cover,
    };
    let max = (u32::try_from(monitor.width).unwrap_or(1920), u32::try_from(monitor.height).unwrap_or(1080));
    let meta = std::fs::metadata(&path).ok();
    let key = WallpaperKey {
        path: path.clone(),
        modified: meta.as_ref().and_then(|m| m.modified().ok()),
        len: meta.as_ref().map_or(0, std::fs::Metadata::len),
        max,
        scaling,
    };
    let mut cache = state.system.wallpaper_cache.lock().await;
    if let Some((cached, url)) = cache.as_ref()
        && *cached == key
    {
        return url.clone();
    }
    let url = tauri::async_runtime::spawn_blocking(move || encode_with_fallback(&path, max, scaling)).await.ok().flatten();
    *cache = Some((key, url.clone()));
    url
}

fn encode_with_fallback(path: &Path, (width, height): (u32, u32), scaling: Scaling) -> Option<String> {
    match imaging::file_to_jpeg_data_url(path, width, height, scaling) {
        Ok(url) => Some(url),
        Err(error) => {
            log::warn!("wallpaper: cannot decode {} ({error})", path.display());
            // HEIC and friends: Windows keeps a JPEG copy.
            let fallback = rice_win::wallpaper::transcoded_fallback().filter(|f| f != path)?;
            imaging::file_to_jpeg_data_url(&fallback, width, height, scaling)
                .inspect_err(|error| log::warn!("wallpaper: fallback failed too ({error})"))
                .ok()
        }
    }
}
