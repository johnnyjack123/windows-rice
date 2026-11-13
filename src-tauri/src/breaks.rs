//! Full-screen pomodoro break windows, one per monitor (docs/BACKEND.md §3.3).
//! The pomodoro runs in the island window; these windows only display it.

use crate::state::{AppState, Locked};
use crate::webview::{BROWSER_ARGS, TITLE};
use rice_core::contract::{BreakOverlay, events};
use rice_core::{BackendError, BackendResult};
use rice_win::monitors::Monitor;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Label prefix: `break-0`, `break-1`, …
pub const PREFIX: &str = "break-";
/// The pages fade out in ~400 ms after `break://state` = null.
const FADE_OUT: Duration = Duration::from_millis(450);

/// Current break and a counter that invalidates pending close timers.
#[derive(Default)]
pub struct Breaks {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    overlay: Option<BreakOverlay>,
    generation: u64,
}

impl Breaks {
    /// The break shown right now, if any.
    pub fn current(&self) -> Option<BreakOverlay> {
        self.inner.locked().overlay.clone()
    }

    /// Whether break windows must refuse to close (Alt+F4).
    pub fn is_active(&self) -> bool {
        self.inner.locked().overlay.is_some()
    }
}

fn label(index: usize) -> String {
    format!("{PREFIX}{index}")
}

/// `break_show`: remembers the state, opens missing windows, announces it.
pub fn show(app: &AppHandle, overlay: BreakOverlay) -> BackendResult<()> {
    {
        let state = app.state::<AppState>();
        let mut inner = state.breaks.inner.locked();
        inner.overlay = Some(overlay.clone());
        inner.generation += 1;
    }
    open_windows(app)?;
    let _ = app.emit(events::BREAK_STATE, Some(overlay));
    Ok(())
}

/// `break_hide`: lets the pages fade out, then closes every break window.
pub fn hide(app: &AppHandle) {
    let generation = {
        let state = app.state::<AppState>();
        let mut inner = state.breaks.inner.locked();
        inner.overlay = None;
        inner.generation += 1;
        inner.generation
    };
    let _ = app.emit(events::BREAK_STATE, None::<BreakOverlay>);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FADE_OUT).await;
        // A new break started meanwhile: keep its windows.
        if handle.state::<AppState>().breaks.inner.locked().generation == generation {
            close_windows(&handle, 0);
        }
    });
}

/// `break_finish`: the user skipped the break in a break window.
pub fn finish(app: &AppHandle) {
    hide(app);
    let _ = app.emit(events::BREAK_FINISHED, ());
}

/// Monitors changed: open windows for new monitors, close those of removed ones.
pub fn sync_monitors(app: &AppHandle) {
    if app.state::<AppState>().breaks.is_active()
        && let Err(error) = open_windows(app)
    {
        log::warn!("break: {error}");
    }
}

fn open_windows(app: &AppHandle) -> BackendResult<()> {
    let monitors = rice_win::monitors::list();
    let island_monitor = app.state::<AppState>().island.placement().map(|p| p.monitor_index);
    for monitor in &monitors {
        let window = match app.get_webview_window(&label(monitor.index)) {
            Some(window) => window,
            None => create_window(app, monitor, island_monitor == Some(monitor.index))?,
        };
        // Cover the whole monitor including the taskbar, in physical pixels.
        let hwnd = window.hwnd().map_err(BackendError::internal)?.0 as isize;
        let rect = monitor.rect;
        app.run_on_main_thread(move || rice_win::window::place_topmost(hwnd, rect)).map_err(BackendError::internal)?;
    }
    close_windows(app, monitors.len());
    Ok(())
}

fn create_window(app: &AppHandle, monitor: &Monitor, focused: bool) -> BackendResult<tauri::WebviewWindow> {
    let state = app.state::<AppState>();
    let r = monitor.rect;
    let scale = monitor.scale;
    WebviewWindowBuilder::new(app, label(monitor.index), WebviewUrl::App("break.html".into()))
        .title(TITLE)
        // Logical values land on the right monitor; place_topmost makes them exact.
        .position(f64::from(r.x) / scale, f64::from(r.y) / scale)
        .inner_size(f64::from(r.width) / scale, f64::from(r.height) / scale)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .focused(focused)
        .owner_raw(rice_win::window::hwnd(state.owner))
        .additional_browser_args(BROWSER_ARGS)
        .build()
        .map_err(BackendError::internal)
}

/// Destroys break windows with an index ≥ `keep` (bypasses the close guard).
fn close_windows(app: &AppHandle, keep: usize) {
    for (name, window) in app.webview_windows() {
        let index = name.strip_prefix(PREFIX).and_then(|i| i.parse::<usize>().ok());
        if index.is_some_and(|i| i >= keep)
            && let Err(error) = window.destroy()
        {
            log::warn!("break: could not close {name}: {error}");
        }
    }
}
