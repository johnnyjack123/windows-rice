//! rice: dynamic island and control centre for Windows (Tauri 2 backend).
//!
//! Contract: `web/src/backend/contract.ts`. Behaviour: `docs/BACKEND.md`.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod blocking;
mod breaks;
mod commands;
mod config;
mod display;
mod hotkey;
mod island;
mod media;
mod power;
mod settings_window;
mod state;
mod sysmon;
mod system;
mod webview;

use rice_core::settings::BackendSettings;
use rice_services::config::ConfigStore;
use rice_services::themes::ThemeStore;
use state::{AppState, Paths};
use std::sync::Mutex;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, Window, WindowEvent};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .plugin(hotkey::plugin())
        .invoke_handler(commands::handler())
        .setup(|app| setup(app.handle()))
        .on_window_event(on_window_event)
        .on_page_load(|webview, payload| {
            // A reloaded page starts without subscriptions.
            if payload.event() == PageLoadEvent::Started {
                sysmon::forget(webview.app_handle(), webview.label());
            }
        })
        .run(tauri::generate_context!());
    if let Err(error) = app {
        eprintln!("rice: {error}");
        std::process::exit(1);
    }
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let paths = Paths::resolve(app)?;
    std::fs::create_dir_all(&paths.themes_dir)?;
    let config = ConfigStore::open(paths.config_file());
    let settings = BackendSettings::from_stored(&config.get());
    let themes = ThemeStore::new(paths.themes_dir.clone());
    let themes_sent = Mutex::new(themes.list());
    // Setup runs on the UI thread, which must own the owner window.
    let owner = rice_win::window::create_owner()?;
    let hotkey = settings.hotkey.clone();
    app.manage(AppState {
        paths,
        config,
        themes,
        themes_sent,
        settings: Mutex::new(settings),
        island: island::Island::default(),
        breaks: breaks::Breaks::default(),
        system: system::SystemState::default(),
        media: media::Media::default(),
        audio: audio::Audio::default(),
        display: display::Display::default(),
        power: power::Power::default(),
        sysmon: sysmon::Monitor::default(),
        owner,
        watchers: Mutex::new(Vec::new()),
    });
    island::create(app)?;
    hotkey::register(app, &hotkey);
    config::start_watchers(app);
    system::start(app)?;
    media::start(app);
    audio::start(app);
    display::start(app);
    log::info!("rice {} started; config in {}", app.package_info().version, app.state::<AppState>().paths.config_dir.display());
    Ok(())
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    let app = window.app_handle();
    let label = window.label();
    match event {
        // Alt+F4 must not close the island, nor a running break (docs/BACKEND.md §3.3).
        WindowEvent::CloseRequested { api, .. }
            if label == island::LABEL
                || (label.starts_with(breaks::PREFIX) && app.state::<AppState>().breaks.is_active()) =>
        {
            api.prevent_close();
        }
        WindowEvent::Focused(false) if label == island::LABEL => island::on_blur(app),
        WindowEvent::ScaleFactorChanged { .. } if label == island::LABEL => island::reposition(app),
        WindowEvent::Destroyed => sysmon::forget(app, label),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::state::Locked;
    use std::sync::Mutex;

    #[test]
    fn poisoned_mutexes_stay_usable() {
        let mutex = Mutex::new(1);
        let _ = std::panic::catch_unwind(|| {
            let _guard = mutex.lock().unwrap();
            panic!("poison");
        });
        assert!(mutex.is_poisoned());
        *mutex.locked() += 1;
        assert_eq!(*mutex.locked(), 2);
    }
}
