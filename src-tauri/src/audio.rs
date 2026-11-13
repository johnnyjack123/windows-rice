//! Output volume, devices and microphone (docs/BACKEND.md §4 "Audio").
//! Every change (also the hardware volume keys) is sent as `audio://state`,
//! which shows the volume OSD in the island.

use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{AudioDevice, AudioState, events};
use rice_core::devices::round_fraction;
use rice_core::{BackendError, BackendResult};
use rice_win::audio::{AudioHandle, AudioSnapshot, Endpoint};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

/// Audio service state.
#[derive(Default)]
pub struct Audio {
    handle: Mutex<Option<AudioHandle>>,
    sent: Mutex<Option<AudioState>>,
}

/// State reported when Core Audio is unavailable.
fn unavailable() -> AudioState {
    AudioState { volume: 0.0, muted: false, device: None, devices: Vec::new(), mic_muted: false, mic_available: false }
}

fn device(endpoint: Endpoint) -> AudioDevice {
    AudioDevice { id: endpoint.id, name: endpoint.name, kind: endpoint.kind }
}

fn to_state(snapshot: AudioSnapshot) -> AudioState {
    AudioState {
        volume: round_fraction(f64::from(snapshot.volume)),
        muted: snapshot.muted,
        device: snapshot.device.map(device),
        devices: snapshot.devices.into_iter().map(device).collect(),
        mic_muted: snapshot.mic_muted,
        mic_available: snapshot.mic_available,
    }
}

/// Starts the audio thread.
pub fn start(app: &AppHandle) {
    let events = app.clone();
    match AudioHandle::start(move |snapshot| {
        let state = to_state(snapshot);
        let changed = events.state::<AppState>().audio.sent.locked().replace(state.clone()).as_ref() != Some(&state);
        if changed {
            let _ = events.emit(events::AUDIO_STATE, state);
        }
    }) {
        Ok(handle) => *app.state::<AppState>().audio.handle.locked() = Some(handle),
        Err(error) => log::warn!("audio: thread not started ({error})"),
    }
}

fn handle(app: &AppHandle) -> BackendResult<AudioHandle> {
    app.state::<AppState>().audio.handle.locked().clone().ok_or_else(|| BackendError::not_supported("audio unavailable"))
}

/// `audio_get`.
pub async fn get(app: &AppHandle) -> BackendResult<AudioState> {
    let Ok(audio) = handle(app) else {
        return Ok(unavailable());
    };
    let state = blocking(move || audio.snapshot()).await.map(to_state).unwrap_or_else(|_| unavailable());
    *app.state::<AppState>().audio.sent.locked() = Some(state.clone());
    Ok(state)
}

/// `audio_set_volume`: also unmutes.
pub async fn set_volume(app: &AppHandle, volume: f64) -> BackendResult<()> {
    if !volume.is_finite() {
        return Err(BackendError::invalid_args("volume must be a number"));
    }
    let audio = handle(app)?;
    blocking(move || audio.set_volume(round_fraction(volume) as f32)).await
}

/// `audio_set_muted`.
pub async fn set_muted(app: &AppHandle, muted: bool) -> BackendResult<()> {
    let audio = handle(app)?;
    blocking(move || audio.set_muted(muted)).await
}

/// `audio_set_mic_muted`.
pub async fn set_mic_muted(app: &AppHandle, muted: bool) -> BackendResult<()> {
    let audio = handle(app)?;
    blocking(move || audio.set_mic_muted(muted)).await
}

/// `audio_set_device`.
pub async fn set_device(app: &AppHandle, id: String) -> BackendResult<()> {
    let audio = handle(app)?;
    blocking(move || audio.set_default_device(id)).await
}
