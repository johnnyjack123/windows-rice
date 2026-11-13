//! Process-wide guard around hiding Windows' notification banners: one
//! backup file, serialised changes, and the safety nets that bring the
//! banners back however rice ends (docs/BACKEND.md §4 "Mitteilungen").
//!
//! Safety nets: `RunEvent::Exit`, `WM_ENDSESSION` (system.rs), Ctrl+C and
//! closing the console (development), a panic hook, `--restore-banners`
//! (uninstaller) and, for a killed process, the next start.

use crate::state::Locked;
use rice_core::notifications::{BannerStore, key_for};
use rice_services::banners::{BACKUP_FILE, restore as restore_from, suppress};
use rice_win::banners::Settings;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock, TryLockError};
use std::time::Duration;

static BACKUP: OnceLock<PathBuf> = OnceLock::new();
/// Serialises registry and backup changes.
static LOCK: Mutex<()> = Mutex::new(());
/// Whether banners are hidden now (new apps follow).
static ACTIVE: AtomicBool = AtomicBool::new(false);
/// Set once rice ends (or panicked): from then on nothing is hidden again.
static ENDED: AtomicBool = AtomicBool::new(false);

/// Remembers the backup location and installs the panic hook and the
/// console handler. Call once, early in setup.
pub fn init(config_dir: &Path) {
    let _ = BACKUP.set(config_dir.join(BACKUP_FILE));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        shutdown();
    }));
    if let Err(error) = rice_win::console::on_close(shutdown) {
        log::debug!("banners: no console handler ({error})");
    }
}

/// `%APPDATA%\rice\banner-backup.json`; also without Tauri (`--restore-banners`).
fn backup_file() -> Option<PathBuf> {
    BACKUP.get().cloned().or_else(|| std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("rice").join(BACKUP_FILE)))
}

/// Whether rice currently hides Windows' banners.
pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

/// Hides the banners of every app that shows them (backup first). Blocking.
pub fn suppress_all() {
    let Some(file) = backup_file() else {
        return;
    };
    let _guard = LOCK.locked();
    if ENDED.load(Ordering::SeqCst) {
        return;
    }
    let settings = Settings::default();
    let count = suppress(&settings, &file, settings.apps());
    ACTIVE.store(true, Ordering::SeqCst);
    log::info!("banners: Windows banners hidden for {count} more apps");
}

/// Hides the banners of an app seen for the first time (its settings key
/// appears with its first notification). Blocking.
pub fn suppress_app(aumid: &str) {
    if !is_active() {
        return;
    }
    let (Some(file), Some(path)) = (backup_file(), key_for(aumid)) else {
        return;
    };
    let _guard = LOCK.locked();
    if ENDED.load(Ordering::SeqCst) || !is_active() {
        return;
    }
    let settings = Settings::default();
    if let Some(app) = settings.app(&path)
        && suppress(&settings, &file, vec![app]) > 0
    {
        log::info!("banners: Windows banners hidden for {aumid}");
    }
}

/// Brings back every banner rice hid that is still hidden. Blocking.
pub fn restore() {
    ACTIVE.store(false, Ordering::SeqCst);
    let Some(file) = backup_file() else {
        return;
    };
    let _guard = lock_briefly();
    let count = restore_from(&Settings::default(), &file);
    if count > 0 {
        log::info!("banners: Windows banners restored for {count} apps");
    }
}

/// Restores and makes sure nothing is hidden again (rice ends).
pub fn shutdown() {
    ENDED.store(true, Ordering::SeqCst);
    restore();
}

/// The lock, unless another thread keeps it for long: a panic in the middle
/// of a change must not block the restore in the panic hook forever.
fn lock_briefly() -> Option<MutexGuard<'static, ()>> {
    for _ in 0..200 {
        match LOCK.try_lock() {
            Ok(guard) => return Some(guard),
            Err(TryLockError::Poisoned(poisoned)) => return Some(poisoned.into_inner()),
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    None
}
