//! Last words before the console ends the process (Ctrl+C, Ctrl+Break or
//! closing the terminal of a development build), which skips every Rust
//! cleanup.

use std::sync::OnceLock;
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::core::BOOL;

static CLEANUP: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// Runs `cleanup` before the console's default handler ends the process.
/// Only the first registration counts.
pub fn on_close(cleanup: impl Fn() + Send + Sync + 'static) -> windows::core::Result<()> {
    if CLEANUP.set(Box::new(cleanup)).is_err() {
        return Ok(());
    }
    // SAFETY: `handler` is a plain function that lives as long as the process.
    unsafe { SetConsoleCtrlHandler(Some(handler), true) }
}

unsafe extern "system" fn handler(_event: u32) -> BOOL {
    if let Some(cleanup) = CLEANUP.get() {
        cleanup();
    }
    // Not handled: the default handler ends the process as usual.
    BOOL(0)
}
