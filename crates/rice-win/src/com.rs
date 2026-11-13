//! COM apartment guard.

use windows::Win32::System::Com::{COINIT, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// Initialises COM on the current thread for its lifetime.
///
/// If the thread already runs a different apartment, COM still works and the
/// guard simply does not uninitialise.
pub struct ComGuard {
    owned: bool,
}

impl ComGuard {
    /// Multithreaded apartment (worker threads).
    pub fn mta() -> Self {
        Self::init(COINIT_MULTITHREADED)
    }

    /// Single-threaded apartment (threads with a message loop, shell calls).
    pub fn sta() -> Self {
        Self::init(COINIT_APARTMENTTHREADED)
    }

    fn init(mode: COINIT) -> Self {
        // SAFETY: plain COM initialisation; balanced in Drop when it succeeded.
        let result = unsafe { CoInitializeEx(None, mode) };
        Self { owned: result.is_ok() }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.owned {
            // SAFETY: balances the successful CoInitializeEx of this guard.
            unsafe { CoUninitialize() };
        }
    }
}
