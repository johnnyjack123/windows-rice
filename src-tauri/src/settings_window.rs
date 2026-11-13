//! The settings window (docs/BACKEND.md §3.2), created on demand.

use crate::webview::{BROWSER_ARGS, TITLE};
use rice_core::shell::is_valid_page;
use rice_core::{BackendError, BackendResult};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// Window label.
pub const LABEL: &str = "settings";

/// `window_open_settings`: focuses the window (creating it if needed) and shows `page`.
pub fn open(app: &AppHandle, page: Option<&str>) -> BackendResult<()> {
    if let Some(page) = page
        && !is_valid_page(page)
    {
        return Err(BackendError::invalid_args(format!("invalid settings page {page:?}")));
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        if let Some(page) = page {
            // The page listens to `hashchange`; no reload needed. `page` is [a-z0-9-] only.
            window.eval(format!("location.hash = '{page}'")).map_err(BackendError::internal)?;
        }
        let _ = window.unminimize();
        let _ = window.show();
        window.set_focus().map_err(BackendError::internal)?;
        return Ok(());
    }
    let url = page.map_or_else(|| "settings.html".to_owned(), |p| format!("settings.html#{p}"));
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(url.into()))
        .title(TITLE)
        .inner_size(1120.0, 780.0)
        .min_inner_size(900.0, 640.0)
        .center()
        .additional_browser_args(BROWSER_ARGS)
        .build()
        .map_err(BackendError::internal)?;
    Ok(())
}
