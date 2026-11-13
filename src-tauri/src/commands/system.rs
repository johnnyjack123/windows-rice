//! Appearance, wallpaper, fonts and monitors.

use crate::{sysmon, system};
use rice_core::BackendResult;
use rice_core::contract::{Appearance, LimitArgs, MonitorInfo, PidArgs, ProcessInfo, SubscribeArgs, WallpaperInfo};
use tauri::{AppHandle, Webview};

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

/// Counted per calling window (see `sysmon`).
#[tauri::command]
pub async fn system_subscribe(app: AppHandle, webview: Webview, args: SubscribeArgs) -> BackendResult<()> {
    sysmon::subscribe(&app, webview.label(), args.interval_ms);
    Ok(())
}

#[tauri::command]
pub async fn system_unsubscribe(app: AppHandle, webview: Webview) -> BackendResult<()> {
    sysmon::unsubscribe(&app, webview.label());
    Ok(())
}

#[tauri::command]
pub async fn system_processes(app: AppHandle, args: LimitArgs) -> BackendResult<Vec<ProcessInfo>> {
    sysmon::processes(&app, args.limit).await
}

#[tauri::command]
pub async fn system_kill(args: PidArgs) -> BackendResult<()> {
    sysmon::kill(args.pid).await
}
