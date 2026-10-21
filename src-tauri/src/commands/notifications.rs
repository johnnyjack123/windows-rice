//! Notification commands (docs/BACKEND.md §6, step 6).

use crate::notifications;
use rice_core::BackendResult;
use rice_core::contract::{IdArgs, NotificationPermission, NotificationsState};
use tauri::AppHandle;

#[tauri::command]
pub async fn notifications_get(app: AppHandle) -> BackendResult<NotificationsState> {
    notifications::get(&app).await
}

#[tauri::command]
pub async fn notifications_request_access(app: AppHandle) -> BackendResult<NotificationPermission> {
    notifications::request_access(&app).await
}

#[tauri::command]
pub async fn notifications_dismiss(app: AppHandle, args: IdArgs) -> BackendResult<()> {
    notifications::dismiss(&app, args.id).await
}

#[tauri::command]
pub async fn notifications_open(app: AppHandle, args: IdArgs) -> BackendResult<()> {
    notifications::open(&app, args.id).await
}

#[tauri::command]
pub async fn notifications_clear(app: AppHandle) -> BackendResult<()> {
    notifications::clear(&app).await
}
