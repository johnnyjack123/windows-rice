//! Global shortcut that toggles the panel (`settings.island.hotkey`).

use rice_core::contract::events;
use tauri::plugin::TauriPlugin;
use tauri::{AppHandle, Emitter, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// The plugin, wired to send `island://toggle` on key press.
pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit(events::ISLAND_TOGGLE, ());
            }
        })
        .build()
}

/// Replaces the registered shortcut. A taken or malformed shortcut is logged,
/// never fatal (another app may own it).
pub fn register(app: &AppHandle, accelerator: &str) {
    let shortcuts = app.global_shortcut();
    if let Err(error) = shortcuts.unregister_all() {
        log::warn!("hotkey: could not release the old shortcut: {error}");
    }
    match shortcuts.register(accelerator) {
        Ok(()) => log::info!("hotkey: {accelerator}"),
        Err(error) => log::warn!("hotkey: {accelerator} not registered: {error}"),
    }
}
