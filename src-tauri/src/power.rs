//! Battery, charger, energy saver and session actions (docs/BACKEND.md §4 "Energie").

use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{PowerAction, PowerState, events};
use rice_core::{BackendError, BackendResult};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

/// Power service state.
#[derive(Default)]
pub struct Power {
    sent: Mutex<Option<PowerState>>,
}

/// `power_get`.
pub fn get(app: &AppHandle) -> PowerState {
    let state = rice_win::power::status();
    *app.state::<AppState>().power.sent.locked() = Some(state);
    state
}

/// Re-reads the power state after a notification; sends `power://state` on change.
pub fn refresh(app: &AppHandle) {
    let state = rice_win::power::status();
    let changed = app.state::<AppState>().power.sent.locked().replace(state) != Some(state);
    if changed {
        let _ = app.emit(events::POWER_STATE, state);
    }
}

/// `power_action`.
pub async fn act(action: PowerAction) -> BackendResult<()> {
    log::info!("power: {action:?}");
    blocking(move || rice_win::power::act(action).map_err(BackendError::internal)).await
}
