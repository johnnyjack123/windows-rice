//! Brightness of the island monitor (docs/BACKEND.md §4 "Anzeige"): WMI for
//! a built-in panel, DDC/CI for external monitors, otherwise unsupported.
//! Night light stays `null`: its only switch is an undocumented registry blob.

use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{DisplayState, events};
use rice_core::devices::{percent, round_fraction};
use rice_core::{BackendError, BackendResult};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Own changes echo back as WMI events; these are not news for the UI.
const ECHO_WINDOW: Duration = Duration::from_millis(1500);

/// How the brightness of the island monitor is controlled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Wmi,
    Ddc(isize),
    Unsupported,
}

/// Display service state.
#[derive(Default)]
pub struct Display {
    /// Resolved source and the monitor it belongs to.
    source: Mutex<Option<(isize, Source)>>,
    last_set: Mutex<Option<(Instant, u8)>>,
    sent: Mutex<DisplayState>,
}

/// Watches the built-in panel's brightness (keys, Windows slider).
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if rice_win::brightness::wmi_get().is_none() {
            return;
        }
        let events = handle.clone();
        let watched = rice_win::brightness::watch_wmi(move |fraction| on_wmi_event(&events, fraction));
        if let Err(error) = watched {
            log::warn!("brightness: watcher not started ({error})");
        }
    });
}

fn on_wmi_event(app: &AppHandle, fraction: f64) {
    if source(app) != Source::Wmi {
        return;
    }
    let display = &app.state::<AppState>().display;
    let echo = display
        .last_set
        .locked()
        .is_some_and(|(at, value)| at.elapsed() < ECHO_WINDOW && value == percent(fraction));
    if !echo {
        send(app, DisplayState { brightness: Some(round_fraction(fraction)), night_light: None });
    }
}

fn send(app: &AppHandle, state: DisplayState) {
    let changed = std::mem::replace(&mut *app.state::<AppState>().display.sent.locked(), state) != state;
    if changed {
        let _ = app.emit(events::DISPLAY_STATE, state);
    }
}

/// Finds out (once per island monitor) how its brightness can be controlled.
/// Blocking: DDC/CI probing takes a while.
fn source(app: &AppHandle) -> Source {
    let state = app.state::<AppState>();
    let Some(placement) = state.island.placement() else {
        return Source::Unsupported;
    };
    if let Some((monitor, source)) = *state.display.source.locked()
        && monitor == placement.monitor_handle
    {
        return source;
    }
    let internal = rice_win::monitors::list()
        .iter()
        .find(|m| m.handle == placement.monitor_handle)
        .is_some_and(|m| m.internal);
    let source = if internal && rice_win::brightness::wmi_get().is_some() {
        Source::Wmi
    } else if rice_win::brightness::ddc_get(placement.monitor_handle).is_some() {
        Source::Ddc(placement.monitor_handle)
    } else {
        Source::Unsupported
    };
    log::info!("brightness: {source:?}");
    *state.display.source.locked() = Some((placement.monitor_handle, source));
    source
}

/// Monitors changed: probe again next time.
pub fn reset(app: &AppHandle) {
    *app.state::<AppState>().display.source.locked() = None;
}

/// `display_get`.
pub async fn get(app: &AppHandle) -> BackendResult<DisplayState> {
    let handle = app.clone();
    blocking(move || {
        let brightness = match source(&handle) {
            Source::Wmi => rice_win::brightness::wmi_get(),
            Source::Ddc(monitor) => rice_win::brightness::ddc_get(monitor),
            Source::Unsupported => None,
        };
        let state = DisplayState { brightness, night_light: None };
        *handle.state::<AppState>().display.sent.locked() = state;
        Ok(state)
    })
    .await
}

/// `display_set_brightness`.
pub async fn set_brightness(app: &AppHandle, value: f64) -> BackendResult<()> {
    if !value.is_finite() {
        return Err(BackendError::invalid_args("brightness must be a number"));
    }
    // Echo exactly what the UI asked for, so its slider sees no "new" value.
    let requested = value.clamp(0.0, 1.0);
    let value = round_fraction(value);
    let handle = app.clone();
    blocking(move || {
        match source(&handle) {
            Source::Wmi => {
                *handle.state::<AppState>().display.last_set.locked() = Some((Instant::now(), percent(value)));
                rice_win::brightness::wmi_set(value).map_err(BackendError::internal)?;
            }
            Source::Ddc(monitor) => {
                if !rice_win::brightness::ddc_set(monitor, value) {
                    return Err(BackendError::internal("monitor refused the brightness"));
                }
            }
            Source::Unsupported => return Err(BackendError::not_supported("brightness is not adjustable on this monitor")),
        }
        send(&handle, DisplayState { brightness: Some(requested), night_light: None });
        Ok(())
    })
    .await
}
