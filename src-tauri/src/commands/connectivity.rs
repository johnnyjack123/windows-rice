//! Wi-Fi, airplane mode, hotspot and Bluetooth commands (docs/BACKEND.md §6, step 4).

use crate::{bluetooth, network};
use rice_core::BackendResult;
use rice_core::contract::{BluetoothState, ConnectResult, DeviceIdArgs, EnabledArgs, NetworkState, WifiConnectArgs, WifiNetwork};
use tauri::AppHandle;

#[tauri::command]
pub async fn network_get(app: AppHandle) -> BackendResult<NetworkState> {
    network::get(&app).await
}

#[tauri::command]
pub async fn network_scan(app: AppHandle) -> BackendResult<Vec<WifiNetwork>> {
    network::scan(&app).await
}

#[tauri::command]
pub async fn network_set_wifi(app: AppHandle, args: EnabledArgs) -> BackendResult<()> {
    network::set_wifi(&app, args.enabled).await
}

#[tauri::command]
pub async fn network_connect(app: AppHandle, args: WifiConnectArgs) -> BackendResult<ConnectResult> {
    network::connect(&app, args.ssid, args.key).await
}

#[tauri::command]
pub async fn network_disconnect(app: AppHandle) -> BackendResult<()> {
    network::disconnect(&app).await
}

#[tauri::command]
pub async fn network_set_airplane(app: AppHandle, args: EnabledArgs) -> BackendResult<()> {
    network::set_airplane(&app, args.enabled).await
}

#[tauri::command]
pub async fn network_set_hotspot(app: AppHandle, args: EnabledArgs) -> BackendResult<()> {
    network::set_hotspot(&app, args.enabled).await
}

#[tauri::command]
pub async fn bluetooth_get(app: AppHandle) -> BackendResult<BluetoothState> {
    bluetooth::get(&app).await
}

#[tauri::command]
pub async fn bluetooth_set_enabled(app: AppHandle, args: EnabledArgs) -> BackendResult<()> {
    bluetooth::set_enabled(&app, args.enabled).await
}

#[tauri::command]
pub async fn bluetooth_connect(app: AppHandle, args: DeviceIdArgs) -> BackendResult<ConnectResult> {
    bluetooth::connect(&app, args.id).await
}

#[tauri::command]
pub async fn bluetooth_disconnect(app: AppHandle, args: DeviceIdArgs) -> BackendResult<()> {
    bluetooth::disconnect(&app, args.id).await
}
