//! Media, audio, display and power commands (docs/BACKEND.md §6, step 2).

use crate::{audio, display, media, power};
use rice_core::contract::{
    AudioState, BrightnessArgs, DeviceIdArgs, DisplayState, EnabledArgs, MediaControlArgs, MediaSeekArgs, MediaState, MutedArgs,
    PowerActionArgs, PowerState, VolumeArgs,
};
use rice_core::{BackendError, BackendResult};
use tauri::AppHandle;

#[tauri::command]
pub async fn media_get(app: AppHandle) -> BackendResult<MediaState> {
    media::get(&app).await
}

#[tauri::command]
pub async fn media_control(app: AppHandle, args: MediaControlArgs) -> BackendResult<()> {
    media::control(&app, args.session_id, args.action).await
}

#[tauri::command]
pub async fn media_seek(app: AppHandle, args: MediaSeekArgs) -> BackendResult<()> {
    media::seek(&app, args.session_id, args.position).await
}

#[tauri::command]
pub async fn audio_get(app: AppHandle) -> BackendResult<AudioState> {
    audio::get(&app).await
}

#[tauri::command]
pub async fn audio_set_volume(app: AppHandle, args: VolumeArgs) -> BackendResult<()> {
    audio::set_volume(&app, args.volume).await
}

#[tauri::command]
pub async fn audio_set_muted(app: AppHandle, args: MutedArgs) -> BackendResult<()> {
    audio::set_muted(&app, args.muted).await
}

#[tauri::command]
pub async fn audio_set_device(app: AppHandle, args: DeviceIdArgs) -> BackendResult<()> {
    audio::set_device(&app, args.id).await
}

#[tauri::command]
pub async fn audio_set_mic_muted(app: AppHandle, args: MutedArgs) -> BackendResult<()> {
    audio::set_mic_muted(&app, args.muted).await
}

#[tauri::command]
pub async fn display_get(app: AppHandle) -> BackendResult<DisplayState> {
    display::get(&app).await
}

#[tauri::command]
pub async fn display_set_brightness(app: AppHandle, args: BrightnessArgs) -> BackendResult<()> {
    display::set_brightness(&app, args.value).await
}

/// Night light has no documented switch; the UI shows it as unsupported (`nightLight: null`).
#[tauri::command]
pub async fn display_set_night_light(args: EnabledArgs) -> BackendResult<()> {
    let _ = args.enabled;
    Err(BackendError::not_supported("night light cannot be switched reliably"))
}

#[tauri::command]
pub async fn power_get(app: AppHandle) -> BackendResult<PowerState> {
    Ok(power::get(&app))
}

/// Energy saver has no documented switch; `energySaverSupported` is false.
#[tauri::command]
pub async fn power_set_energy_saver(args: EnabledArgs) -> BackendResult<()> {
    let _ = args.enabled;
    Err(BackendError::not_supported("the energy saver cannot be switched reliably"))
}

#[tauri::command]
pub async fn power_action(args: PowerActionArgs) -> BackendResult<()> {
    power::act(args.action).await
}
