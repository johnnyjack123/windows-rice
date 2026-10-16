//! `ShellExecuteW` for whitelisted targets (the whitelist lives in rice-core).

use crate::com::ComGuard;
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
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
