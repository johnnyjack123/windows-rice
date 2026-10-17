//! Reacting to settings and theme changes, wherever they come from
//! (`config_set`, an editor, a file copied into the themes folder).

use crate::hotkey;
use crate::island;
use crate::notifications;
use crate::state::{AppState, Locked};
use rice_core::contract::events;
use rice_core::settings::BackendSettings;
use rice_services::watch::{touches_file, watch};
use serde_json::Value;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const CONFIG_DEBOUNCE: Duration = Duration::from_millis(200);
const THEMES_DEBOUNCE: Duration = Duration::from_millis(250);

/// Sends `config://changed` and applies what the backend itself uses.
pub fn on_config_changed(app: &AppHandle, stored: &Value) {
    let _ = app.emit(events::CONFIG_CHANGED, stored);
    let state = app.state::<AppState>();
    let next = BackendSettings::from_stored(stored);
    let previous = std::mem::replace(&mut *state.settings.locked(), next.clone());
    if previous.monitor != next.monitor {
        island::reposition(app);
    } else if previous.offset_top != next.offset_top {
        // The island zone grows with the offset.
        island::refresh_occlusion(app);
    }
    if previous.hotkey != next.hotkey {
        hotkey::register(app, &next.hotkey);
    }
    if previous.hide_windows_banners != next.hide_windows_banners {
        notifications::on_setting_changed(app);
    }
}

/// Sends `theme://user-changed` with the current list.
pub fn emit_themes(app: &AppHandle) {
    let state = app.state::<AppState>();
    let list = state.themes.list();
    *state.themes_sent.locked() = list.clone();
    let _ = app.emit(events::THEMES_CHANGED, list);
}

/// Watches `config.json` and the themes folder for external edits.
pub fn start_watchers(app: &AppHandle) {
    let state = app.state::<AppState>();
    let handle = app.clone();
    let config = watch(
        &state.paths.config_dir,
        false,
        CONFIG_DEBOUNCE,
        |event| touches_file(event, "config.json"),
        move || {
            if let Some(stored) = handle.state::<AppState>().config.reload() {
                on_config_changed(&handle, &stored);
            }
        },
    );
    let handle = app.clone();
    let themes = watch(&state.paths.themes_dir, true, THEMES_DEBOUNCE, |_| true, move || {
        let state = handle.state::<AppState>();
        // Own saves already announced the list; only report real differences.
        if state.themes.list() != *state.themes_sent.locked() {
            emit_themes(&handle);
        }
    });
    let mut watchers = state.watchers.locked();
    for (name, result) in [("config", config), ("themes", themes)] {
        match result {
            Ok(guard) => watchers.push(guard),
            Err(error) => log::warn!("watch {name}: {error}"),
        }
    }
}
