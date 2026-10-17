//! Battery, charger, energy saver and session actions.

use rice_core::contract::{PowerAction, PowerState};
use rice_core::devices::power_state;
use windows::Win32::Foundation::{CloseHandle, HANDLE, LUID};
use windows::Win32::Security::{
    AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS, SetSuspendState};
use windows::Win32::System::Shutdown::{
    EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, ExitWindowsEx, LockWorkStation, SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER,
};
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

/// Locks, sleeps, signs out, restarts or shuts down.
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
        }
    }
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
