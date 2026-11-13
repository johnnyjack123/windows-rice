//! Calendar and task commands (docs/BACKEND.md §6, step 5).

use crate::calendar;
use rice_core::BackendResult;
use rice_core::contract::{
    AddAccountArgs, CalendarAccount, CalendarEnabledArgs, CalendarEvent, IdArgs, RangeArgs, Task, TaskCreateArgs, TaskUpdateArgs,
};
use tauri::AppHandle;

#[tauri::command]
pub async fn calendar_accounts(app: AppHandle) -> BackendResult<Vec<CalendarAccount>> {
    Ok(calendar::accounts(&app))
}

#[tauri::command]
pub async fn calendar_add_account(app: AppHandle, args: AddAccountArgs) -> BackendResult<CalendarAccount> {
    calendar::add_account(&app, args.name, args.url, args.user, args.password).await
}

#[tauri::command]
pub async fn calendar_remove_account(app: AppHandle, args: IdArgs) -> BackendResult<()> {
    calendar::remove_account(&app, &args.id)
}

#[tauri::command]
pub async fn calendar_set_enabled(app: AppHandle, args: CalendarEnabledArgs) -> BackendResult<()> {
    calendar::set_enabled(&app, &args.calendar_id, args.enabled)
}

#[tauri::command]
pub async fn calendar_sync(app: AppHandle) -> BackendResult<()> {
    calendar::sync_all(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn calendar_events(app: AppHandle, args: RangeArgs) -> BackendResult<Vec<CalendarEvent>> {
    calendar::events(&app, &args.from, &args.to).await
}

#[tauri::command]
pub async fn tasks_list(app: AppHandle) -> BackendResult<Vec<Task>> {
    calendar::tasks(&app).await
}

#[tauri::command]
pub async fn tasks_create(app: AppHandle, args: TaskCreateArgs) -> BackendResult<Task> {
    calendar::create_task(&app, &args.calendar_id, &args.title, args.due.as_deref()).await
}

#[tauri::command]
pub async fn tasks_update(app: AppHandle, args: TaskUpdateArgs) -> BackendResult<Task> {
    calendar::update_task(&app, &args.id, args.patch).await
}

#[tauri::command]
pub async fn tasks_delete(app: AppHandle, args: IdArgs) -> BackendResult<()> {
    calendar::delete_task(&app, &args.id).await
}
