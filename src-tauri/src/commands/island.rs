//! Island window commands (docs/BACKEND.md §3.1).

use crate::island;
use rice_core::BackendResult;
use rice_core::contract::{EnabledArgs, ExpandedArgs, HitRegionsArgs, IslandConfigureArgs, Occlusion};
use tauri::AppHandle;

#[tauri::command]
pub async fn island_configure(app: AppHandle, args: IslandConfigureArgs) -> BackendResult<()> {
    island::configure(&app, args.width, args.height)
}

#[tauri::command]
pub async fn island_set_hit_regions(app: AppHandle, args: HitRegionsArgs) -> BackendResult<()> {
    island::set_regions(&app, args.rects);
    Ok(())
}

#[tauri::command]
pub async fn island_set_expanded(app: AppHandle, args: ExpandedArgs) -> BackendResult<()> {
    island::set_expanded(&app, args.expanded)
}

#[tauri::command]
pub async fn island_track_cursor(app: AppHandle, args: EnabledArgs) -> BackendResult<()> {
    island::track_cursor(&app, args.enabled);
    Ok(())
}

#[tauri::command]
pub async fn island_occlusion(app: AppHandle) -> BackendResult<Occlusion> {
    Ok(island::refresh_occlusion(&app))
}
