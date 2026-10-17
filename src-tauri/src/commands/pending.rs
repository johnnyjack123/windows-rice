//! Commands of later steps (docs/BACKEND.md §6). Until a step is built, they
//! report "nothing here": empty or unsupported states, so the UI shows its
//! fallbacks, and `not_supported` for actions. Each step replaces its block.

use rice_core::{BackendError, BackendResult};
use serde_json::{Value, json};

fn later(step: u8) -> BackendError {
    BackendError::not_supported(format!("not built yet (docs/BACKEND.md §6, step {step})"))
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
