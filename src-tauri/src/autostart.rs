//! Starting with Windows (per-user `Run` key, docs/BACKEND.md §7, step 7).
//!
//! Only release builds register themselves: a development build would start
//! `target\debug\rice.exe` at sign-in, which needs the Vite server.

use crate::blocking::blocking;
use rice_core::contract::AutostartState;
use rice_core::{BackendError, BackendResult};
use rice_win::autostart::Autostart;

/// Value name in `Run` (the installer and uninstaller use the same).
const NAME: &str = "rice";

fn supported() -> bool {
    !cfg!(debug_assertions)
}

/// The command line Windows runs at sign-in.
fn command() -> BackendResult<String> {
    let exe = std::env::current_exe().map_err(BackendError::internal)?;
    Ok(format!("\"{}\"", exe.display()))
}

fn state() -> AutostartState {
    AutostartState { enabled: supported() && Autostart::new(NAME).is_enabled(), supported: supported() }
}

/// `app_autostart_get`.
pub async fn get() -> BackendResult<AutostartState> {
    blocking(|| Ok(state())).await
}

/// `app_autostart_set`.
pub async fn set(enabled: bool) -> BackendResult<AutostartState> {
    if !supported() {
        return Err(BackendError::not_supported("only the installed app can start with Windows"));
    }
    blocking(move || {
        let entry = Autostart::new(NAME);
        if enabled {
            entry.enable(&command()?)
        } else {
            entry.disable()
        }
        .map_err(BackendError::internal)?;
        Ok(state())
    })
    .await
}

/// After an update into another folder the entry would point to the old
/// exe: keep an existing entry on the current one (Task Manager's switch
/// stays as it is).
pub fn repair() {
    if !supported() {
        return;
    }
    let entry = Autostart::new(NAME);
    let Ok(current) = command() else {
        return;
    };
    if let Some(registered) = entry.command()
        && !registered.eq_ignore_ascii_case(&current)
        && let Err(error) = entry.set_command(&current)
    {
        log::warn!("autostart: cannot update the entry ({error})");
    }
}
