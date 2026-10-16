//! Appearance, wallpaper, fonts and monitors.

use crate::system;
use rice_core::BackendResult;
use rice_core::contract::{Appearance, MonitorInfo, WallpaperInfo};
use tauri::AppHandle;

#[tauri::command]
pub async fn system_appearance() -> BackendResult<Appearance> {
    Ok(system::appearance())
}

#[tauri::command]
pub async fn system_wallpaper(app: AppHandle) -> BackendResult<WallpaperInfo> {
    system::wallpaper(&app).await
}

#[tauri::command]
pub async fn system_fonts() -> BackendResult<Vec<String>> {
    system::fonts().await
}

#[tauri::command]
pub async fn system_monitors() -> BackendResult<Vec<MonitorInfo>> {
    Ok(system::monitors())
}
