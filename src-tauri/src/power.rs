//! Battery, charger, energy saver, pending updates and session actions
//! (docs/BACKEND.md §4 "Energie").

use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{PowerAction, PowerState, events};
use rice_core::{BackendError, BackendResult};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Windows Update is asked at most this often.
const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(60);

/// Power service state.
#[derive(Default)]
pub struct Power {
    sent: Mutex<Option<PowerState>>,
    /// Last answer of Windows Update ("restart pending").
    update_pending: AtomicBool,
    update_checked: Mutex<Option<Instant>>,
}

fn current(app: &AppHandle) -> PowerState {
    let mut state = rice_win::power::status();
    state.update_pending = app.state::<AppState>().power.update_pending.load(Ordering::SeqCst);
    state
}

/// `power_get`.
pub fn get(app: &AppHandle) -> PowerState {
    let state = current(app);
    *app.state::<AppState>().power.sent.locked() = Some(state);
    state
}

/// Re-reads the power state after a notification; sends `power://state` on change.
pub fn refresh(app: &AppHandle) {
    let state = current(app);
    let changed = app.state::<AppState>().power.sent.locked().replace(state) != Some(state);
    if changed {
        let _ = app.emit(events::POWER_STATE, state);
    }
}

/// Asks Windows Update whether a restart is pending. Windows has no event for
/// it, so rice asks at start and whenever the panel opens (where the power
/// menu shows it), at most once a minute.
pub fn check_updates(app: &AppHandle) {
    {
        let state = app.state::<AppState>();
        let mut checked = state.power.update_checked.locked();
        if checked.is_some_and(|at| at.elapsed() < UPDATE_CHECK_INTERVAL) {
            return;
        }
        *checked = Some(Instant::now());
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || match rice_win::power::update_pending() {
        Ok(pending) => {
            handle.state::<AppState>().power.update_pending.store(pending, Ordering::SeqCst);
            refresh(&handle);
        }
        Err(error) => log::debug!("power: Windows Update state unreadable ({error})"),
    });
}

/// `power_action`.
pub async fn act(action: PowerAction) -> BackendResult<()> {
    log::info!("power: {action:?}");
    blocking(move || rice_win::power::act(action).map_err(BackendError::internal)).await
}
