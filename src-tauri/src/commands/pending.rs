//! Commands of later steps (docs/BACKEND.md §6). Until a step is built, they
//! report "nothing here": empty or unsupported states, so the UI shows its
//! fallbacks, and `not_supported` for actions. Each step replaces its block.

use rice_core::{BackendError, BackendResult};
use serde_json::{Value, json};

fn later(step: u8) -> BackendError {
    BackendError::not_supported(format!("not built yet (docs/BACKEND.md §6, step {step})"))
}

// ── Step 3: system monitor ───────────────────────────────────────────────

#[tauri::command]
pub async fn system_subscribe() -> BackendResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn system_unsubscribe() -> BackendResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn system_processes() -> BackendResult<Value> {
    Ok(json!([]))
}

#[tauri::command]
pub async fn system_kill() -> BackendResult<()> {
    Err(later(3))
}

// ── Step 4: network, Bluetooth ───────────────────────────────────────────

#[tauri::command]
pub async fn network_get() -> BackendResult<Value> {
    Ok(json!({
        "wifi": { "available": false, "enabled": false, "connected": null, "permission": "granted" },
        "airplane": false,
        "hotspot": { "supported": false, "enabled": false },
    }))
}

#[tauri::command]
pub async fn network_scan() -> BackendResult<Value> {
    Ok(json!([]))
}

#[tauri::command]
pub async fn network_set_wifi() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn network_connect() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn network_disconnect() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn network_set_airplane() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn network_set_hotspot() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn bluetooth_get() -> BackendResult<Value> {
    Ok(json!({ "available": false, "enabled": false, "devices": [] }))
}

#[tauri::command]
pub async fn bluetooth_set_enabled() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn bluetooth_connect() -> BackendResult<()> {
    Err(later(4))
}

#[tauri::command]
pub async fn bluetooth_disconnect() -> BackendResult<()> {
    Err(later(4))
}

// ── Step 5: calendar, tasks ──────────────────────────────────────────────

#[tauri::command]
pub async fn calendar_accounts() -> BackendResult<Value> {
    Ok(json!([]))
}

#[tauri::command]
pub async fn calendar_add_account() -> BackendResult<()> {
    Err(later(5))
}

#[tauri::command]
pub async fn calendar_remove_account() -> BackendResult<()> {
    Err(later(5))
}

#[tauri::command]
pub async fn calendar_set_enabled() -> BackendResult<()> {
    Err(later(5))
}

#[tauri::command]
pub async fn calendar_sync() -> BackendResult<()> {
    Ok(())
}

#[tauri::command]
pub async fn calendar_events() -> BackendResult<Value> {
    Ok(json!([]))
}

#[tauri::command]
pub async fn tasks_list() -> BackendResult<Value> {
    Ok(json!([]))
}

#[tauri::command]
pub async fn tasks_create() -> BackendResult<()> {
    Err(later(5))
}

#[tauri::command]
pub async fn tasks_update() -> BackendResult<()> {
    Err(later(5))
}

#[tauri::command]
pub async fn tasks_delete() -> BackendResult<()> {
    Err(later(5))
}

// ── Step 6: notifications ────────────────────────────────────────────────

#[tauri::command]
pub async fn notifications_get() -> BackendResult<Value> {
    Ok(json!({ "permission": "unsupported", "items": [] }))
}

#[tauri::command]
pub async fn notifications_request_access() -> BackendResult<Value> {
    Ok(json!("unsupported"))
}

#[tauri::command]
pub async fn notifications_dismiss() -> BackendResult<()> {
    Err(later(6))
}

#[tauri::command]
pub async fn notifications_clear() -> BackendResult<()> {
    Ok(())
}
