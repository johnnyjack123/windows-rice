//! `ShellExecute` for whitelisted targets (the whitelist lives in rice-core)
//! and for starting apps by AUMID.

use crate::com::ComGuard;
use windows::Win32::UI::Shell::{SEE_MASK_FLAG_NO_UI, SHELLEXECUTEINFOW, ShellExecuteExW, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{ASFW_ANY, AllowSetForegroundWindow, SW_SHOWNORMAL};
use windows::core::{HSTRING, PCWSTR, w};

/// Opens a URI or path with its default handler.
pub fn open(target: &str) -> windows::core::Result<()> {
    // Shell extensions may need an STA.
    let _com = ComGuard::sta();
    let target = HSTRING::from(target);
    // SAFETY: NUL-terminated strings that outlive the call.
    let result = unsafe { ShellExecuteW(None, w!("open"), &target, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL) };
    // Values above 32 mean success (documented HINSTANCE convention).
    if result.0 as isize > 32 { Ok(()) } else { Err(windows::core::Error::from_thread()) }
}

/// Starts (or brings forward) the app registered under `aumid`, the way the
/// Start menu does (`shell:AppsFolder\<AUMID>`). Fails without any dialog
/// if Windows knows no such app. The caller validates the AUMID.
pub fn open_app(aumid: &str) -> windows::core::Result<()> {
    let _com = ComGuard::sta();
    // The user just clicked in rice: let the app that answers take the foreground.
    // SAFETY: plain call; fails harmlessly when rice may not grant it.
    let _ = unsafe { AllowSetForegroundWindow(ASFW_ANY) };
    let target = HSTRING::from(format!(r"shell:AppsFolder\{aumid}"));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_FLAG_NO_UI,
        lpVerb: w!("open"),
        lpFile: PCWSTR(target.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: the struct and its strings outlive the call.
    unsafe { ShellExecuteExW(&mut info) }
}
