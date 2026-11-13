//! rice: dynamic island and control centre for Windows (Tauri 2 backend).
//!
//! Contract: `web/src/backend/contract.ts`. Behaviour: `docs/BACKEND.md`.

// No console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent_hook;
mod agents;
mod audio;
mod autostart;
mod banners;
mod blocking;
mod bluetooth;
mod breaks;
mod calendar;
mod commands;
mod config;
mod display;
mod hotkey;
mod island;
mod media;
mod network;
mod notifications;
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
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, RunEvent, Window, WindowEvent};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    // For coding agents: forward one hook call to the running rice (exits with 0).
    if let Some(position) = args.iter().position(|arg| arg == rice_core::agents::HOOK_FLAG) {
        agent_hook::run(args.get(position + 1).map_or("", String::as_str));
        return;
    }
    // For the uninstaller: take rice's hooks out of the agents' settings.
    if flag("--disconnect-agents") {
        agents::disconnect_all();
        return;
    }
    // For the installer: end a running rice in an orderly way (its banners come back).
    if flag("--quit") {
        rice_win::events::request_quit(Duration::from_secs(5));
        return;
    }
    // For the uninstaller (and by hand): bring Windows' banners back, then quit.
    if flag("--restore-banners") {
        banners::shutdown();
        return;
    }
    // Development aid: what the notification listener sees, without a window.
    if flag("--check-notifications") {
        notifications::print_diagnosis();
        return;
    }
    let builder = tauri::Builder::default();
    // The installed rice runs once; starting it again opens the settings. Development
    // builds skip this, so `pnpm tauri dev` works while the installed rice runs.
    #[cfg(not(debug_assertions))]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Err(error) = settings_window::open(app, None) {
            log::warn!("single instance: cannot open the settings ({error:?})");
        }
    }));
    let app = builder
        .plugin(tauri_plugin_log::Builder::new().level(log::LevelFilter::Info).build())
        .plugin(hotkey::plugin())
        .invoke_handler(commands::handler())
        .setup(|app| setup(app.handle()))
        .on_window_event(on_window_event)
        .on_page_load(|webview, payload| {
            // A reloaded page starts without subscriptions.
            if payload.event() == PageLoadEvent::Started {
                sysmon::forget(webview.app_handle(), webview.label());
                if webview.label() == island::LABEL {
                    island::track_cursor(webview.app_handle(), false);
                }
            }
        })
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|_, event| {
            if let RunEvent::Exit = event {
                banners::shutdown();
            }
        }),
        Err(error) => {
            eprintln!("rice: {error}");
            std::process::exit(1);
        }
    }
}

fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let paths = Paths::resolve(app)?;
    banners::init(&paths.config_dir);
    std::fs::create_dir_all(&paths.themes_dir)?;
    let config = ConfigStore::open(paths.config_file());
    let settings = BackendSettings::from_stored(&config.get());
    let themes = ThemeStore::new(paths.themes_dir.clone());
    let themes_sent = Mutex::new(themes.list());
    let calendars = calendar::Calendars::open(&paths.config_dir)?;
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
        network: network::Network::default(),
        bluetooth: bluetooth::Bluetooth::default(),
        notifications: notifications::Notifications::default(),
        agents: agents::Agents::default(),
        calendar: calendars,
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
    network::start(app);
    bluetooth::start(app);
    calendar::start(app);
    notifications::start(app);
    agents::start(app);
    power::check_updates(app);
    autostart::repair();
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
