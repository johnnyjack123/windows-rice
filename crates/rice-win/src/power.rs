//! Battery, charger, energy saver, pending Windows updates and session
//! actions.

use crate::com::ComGuard;
use rice_core::contract::{PowerAction, PowerState};
use rice_core::devices::power_state;
use windows::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, LUID, WIN32_ERROR};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS, SetSuspendState};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Shutdown::{
    EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, ExitWindowsEx, InitiateShutdownW, LockWorkStation, SHTDN_REASON_FLAG_PLANNED,
    SHTDN_REASON_MAJOR_OPERATINGSYSTEM, SHTDN_REASON_MAJOR_OTHER, SHTDN_REASON_MINOR_UPGRADE, SHUTDOWN_FLAGS,
    SHUTDOWN_INSTALL_UPDATES, SHUTDOWN_POWEROFF, SHUTDOWN_RESTART,
};
use windows::Win32::System::UpdateAgent::{ISystemInformation, SystemInformation};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::PCWSTR;

/// Current battery, charger and energy saver state.
pub fn status() -> PowerState {
    let mut s = SYSTEM_POWER_STATUS::default();
    // SAFETY: out-pointer to a local.
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() {
        return power_state(255, 255, 255, 0);
    }
    power_state(s.ACLineStatus, s.BatteryFlag, s.BatteryLifePercent, s.SystemStatusFlag)
}

/// Whether Windows Update waits for a restart to finish installing
/// (Windows Update Agent, `ISystemInformation::RebootRequired`).
pub fn update_pending() -> windows::core::Result<bool> {
    let _com = ComGuard::mta();
    // SAFETY: in-process COM object of the Windows Update Agent; plain property read.
    unsafe {
        let info: ISystemInformation = CoCreateInstance(&SystemInformation, None, CLSCTX_INPROC_SERVER)?;
        Ok(info.RebootRequired()?.as_bool())
    }
}

/// Locks, sleeps, signs out, restarts or shuts down (optionally installing
/// pending updates first, like "Update and restart" in the Start menu).
pub fn act(action: PowerAction) -> windows::core::Result<()> {
    // SAFETY: documented session APIs without pointer arguments beyond locals.
    unsafe {
        match action {
            PowerAction::Lock => LockWorkStation(),
            PowerAction::Sleep => {
                if SetSuspendState(false, false, false) {
                    Ok(())
                } else {
                    Err(windows::core::Error::from_thread())
                }
            }
            PowerAction::Signout => ExitWindowsEx(EWX_LOGOFF, SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED),
            PowerAction::Restart => {
                enable_privilege(SE_SHUTDOWN_NAME)?;
                ExitWindowsEx(EWX_REBOOT, SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED)
            }
            PowerAction::Shutdown => {
                enable_privilege(SE_SHUTDOWN_NAME)?;
                ExitWindowsEx(EWX_POWEROFF, SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_FLAG_PLANNED)
            }
            PowerAction::UpdateRestart => shutdown_installing_updates(SHUTDOWN_RESTART),
            PowerAction::UpdateShutdown => shutdown_installing_updates(SHUTDOWN_POWEROFF),
        }
    }
}

/// `InitiateShutdownW` with `SHUTDOWN_INSTALL_UPDATES`: Windows installs the
/// pending updates, then restarts or powers off.
fn shutdown_installing_updates(mode: SHUTDOWN_FLAGS) -> windows::core::Result<()> {
    enable_privilege(SE_SHUTDOWN_NAME)?;
    let reason = SHTDN_REASON_MAJOR_OPERATINGSYSTEM | SHTDN_REASON_MINOR_UPGRADE | SHTDN_REASON_FLAG_PLANNED;
    // SAFETY: local machine (null name), no message, no grace period.
    let status = WIN32_ERROR(unsafe { InitiateShutdownW(PCWSTR::null(), PCWSTR::null(), 0, mode | SHUTDOWN_INSTALL_UPDATES, reason) });
    if status == ERROR_SUCCESS { Ok(()) } else { Err(status.to_hresult().into()) }
}


/// Enables a privilege the user holds but the process token has disabled.
fn enable_privilege(name: PCWSTR) -> windows::core::Result<()> {
    let mut token = HANDLE::default();
    // SAFETY: token of our own process; closed below.
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut token)?;
        let mut luid = LUID::default();
        let result = LookupPrivilegeValueW(PCWSTR::null(), name, &mut luid).and_then(|()| {
            let privileges = TOKEN_PRIVILEGES {
                PrivilegeCount: 1,
                Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
            };
            AdjustTokenPrivileges(token, false, Some(&privileges), 0, None, None)
        });
        let _ = CloseHandle(token);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_windows_update_state() {
        // Either answer is fine; the agent must answer without error.
        assert!(update_pending().is_ok());
    }
}
