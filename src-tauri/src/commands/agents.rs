//! Coding-agent commands (docs/BACKEND.md §4 "Coding-Agents").

use crate::agents;
use rice_core::BackendResult;
use rice_core::contract::{AgentIdArgs, AgentIntegration, AgentRequest, AgentRespondArgs};
use tauri::AppHandle;

#[tauri::command]
pub async fn agents_pending(app: AppHandle) -> BackendResult<Vec<AgentRequest>> {
    Ok(agents::pending(&app))
}

#[tauri::command]
pub async fn agents_respond(app: AppHandle, args: AgentRespondArgs) -> BackendResult<()> {
    agents::respond(&app, &args.id, args.decision)
}

#[tauri::command]
pub async fn agents_list() -> BackendResult<Vec<AgentIntegration>> {
    Ok(agents::list())
}

#[tauri::command]
pub async fn agents_connect(args: AgentIdArgs) -> BackendResult<AgentIntegration> {
    agents::connect(args.id)
}

#[tauri::command]
pub async fn agents_disconnect(args: AgentIdArgs) -> BackendResult<AgentIntegration> {
    agents::disconnect(args.id)
}
