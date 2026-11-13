//! Running blocking Windows calls (WinRT `join`, COM round trips, DDC/CI)
//! off the async runtime's worker threads.

use rice_core::{BackendError, BackendResult};

/// Runs `work` on the blocking pool and flattens the join error.
pub async fn blocking<T: Send + 'static>(work: impl FnOnce() -> BackendResult<T> + Send + 'static) -> BackendResult<T> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(BackendError::internal)?
}
