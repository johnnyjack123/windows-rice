//! Shared application state (managed by Tauri, reachable from every thread
//! through `AppHandle::state`).

use crate::breaks::Breaks;
use crate::island::Island;
use crate::system::SystemState;
use rice_core::contract::UserTheme;
use rice_core::settings::BackendSettings;
use rice_services::config::ConfigStore;
use rice_services::themes::ThemeStore;
use rice_services::watch::WatchGuard;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager, Runtime};

/// Where rice keeps its files.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `%APPDATA%\rice`
    pub config_dir: PathBuf,
    /// `%APPDATA%\rice\themes`
    pub themes_dir: PathBuf,
}

impl Paths {
    /// Resolves the folders below the roaming app data directory.
    pub fn resolve<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Self> {
        let config_dir = app.path().config_dir()?.join("rice");
        let themes_dir = config_dir.join("themes");
        Ok(Self { config_dir, themes_dir })
    }

    /// `config.json`
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }
}

/// Everything the commands, watchers and background threads share.
pub struct AppState {
    pub paths: Paths,
    pub config: ConfigStore,
    pub themes: ThemeStore,
    /// Last theme list sent with `theme://user-changed`.
    pub themes_sent: Mutex<Vec<UserTheme>>,
    /// The settings the backend acts on, derived from `config`.
    pub settings: Mutex<BackendSettings>,
    pub island: Island,
    pub breaks: Breaks,
    pub system: SystemState,
    /// Invisible owner of the island and break windows (keeps them out of Alt+Tab).
    pub owner: isize,
    pub watchers: Mutex<Vec<WatchGuard>>,
}

/// `Mutex::lock` that survives poisoning: a panicked thread must not take
/// every later command down with it.
pub trait Locked<T> {
    /// Locks, recovering the data of a poisoned mutex.
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> Locked<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
