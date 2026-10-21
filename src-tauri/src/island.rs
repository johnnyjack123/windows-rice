//! The island window (docs/BACKEND.md §3.1): a fixed, transparent, topmost
//! window at the top centre of one monitor. The island morphs inside the
//! webview; the backend handles click-through, focus, overlap and the edge.

use crate::state::{AppState, Locked};
use crate::system;
use crate::webview::{BROWSER_ARGS, TITLE};
use rice_core::contract::{EdgeState, Occlusion, Rect, Size, events};
use rice_core::edge::{EdgeInput, EdgeTracker};
use rice_core::geometry::{self, PhysPoint, PhysRect};
use rice_core::settings::MonitorSelection;
use rice_core::{BackendError, BackendResult};
use rice_win::monitors::Monitor;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Window label.
pub const LABEL: &str = "island";

/// Size until the frontend calls `island_configure` (its own fallback values).
const DEFAULT_SIZE: Size = Size { width: 896.0, height: 662.0 };
/// Cursor polling interval (~60 Hz, docs/BACKEND.md §3.1).
const TICK: Duration = Duration::from_millis(16);
/// Grace period before a blur counts as "focus lost": focus moving into the
/// webview's child window also blurs the top-level window briefly.
const BLUR_GRACE: Duration = Duration::from_millis(40);

/// Where the island window sits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub monitor_index: usize,
    pub monitor_handle: isize,
    pub monitor: PhysRect,
    pub work: PhysRect,
    pub scale: f64,
    pub window: PhysRect,
}

/// Island window state shared by commands, the cursor thread and system events.
#[derive(Default)]
pub struct Island {
    inner: Mutex<Inner>,
}

struct Inner {
    hwnd: isize,
    size: Size,
    regions: Vec<Rect>,
    expanded: bool,
    /// Window that had the keyboard before the panel opened.
    previous_foreground: isize,
    placement: Option<Placement>,
    occlusion: Occlusion,
    edge: EdgeTracker,
    /// Whether the window currently takes mouse input.
    interactive: bool,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            hwnd: 0,
            size: DEFAULT_SIZE,
            regions: Vec::new(),
            expanded: false,
            previous_foreground: 0,
            placement: None,
            occlusion: Occlusion::default(),
            edge: EdgeTracker::default(),
            interactive: false,
        }
    }
}

impl Island {
    /// Current placement, if the window exists.
    pub fn placement(&self) -> Option<Placement> {
        self.inner.locked().placement
    }
}

/// Creates the island window and starts the cursor thread.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("island.html".into()))
        .title(TITLE)
        .inner_size(DEFAULT_SIZE.width, DEFAULT_SIZE.height)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        // Compact: WS_EX_NOACTIVATE, never steals the keyboard.
        .focused(false)
        .focusable(false)
        .owner_raw(rice_win::window::hwnd(state.owner))
        .additional_browser_args(BROWSER_ARGS)
        .build()?;
    // Fully click-through until the frontend reports hit regions.
    window.set_ignore_cursor_events(true)?;
    state.island.inner.locked().hwnd = window.hwnd()?.0 as isize;
    reposition(app);
    spawn_cursor_thread(app.clone())?;
    Ok(())
}

fn window(app: &AppHandle) -> BackendResult<WebviewWindow> {
    app.get_webview_window(LABEL).ok_or_else(|| BackendError::not_found("island window"))
}

/// The monitor the settings ask for; falls back to the primary one.
pub fn pick_monitor(monitors: &[Monitor], selection: MonitorSelection) -> Option<&Monitor> {
    let primary = || monitors.iter().find(|m| m.primary).or_else(|| monitors.first());
    match selection {
        MonitorSelection::Index(i) => monitors.get(i).or_else(primary),
        MonitorSelection::Primary => primary(),
    }
}

/// Recomputes where the window goes (monitor, DPI, work area, size) and moves
/// it there. Sends `system://wallpaper` and re-checks overlap when that changed.
pub fn reposition(app: &AppHandle) {
    let state = app.state::<AppState>();
    let selection = state.settings.locked().monitor;
    let monitors = rice_win::monitors::list();
    let Some(monitor) = pick_monitor(&monitors, selection) else {
        log::warn!("island: no monitor found");
        return;
    };
    let (hwnd, placement, changed, edge_reset) = {
        let mut inner = state.island.inner.locked();
        let window = geometry::island_window_rect(monitor.rect, monitor.work, monitor.scale, inner.size);
        let placement = Placement {
            monitor_index: monitor.index,
            monitor_handle: monitor.handle,
            monitor: monitor.rect,
            work: monitor.work,
            scale: monitor.scale,
            window,
        };
        let previous = inner.placement.replace(placement);
        let moved_monitor = previous.is_some_and(|p| p.monitor != placement.monitor);
        let edge_reset = if moved_monitor { inner.edge.reset() } else { None };
        (inner.hwnd, placement, previous != Some(placement), edge_reset)
    };
    if hwnd == 0 {
        return;
    }
    if let Some(active) = edge_reset {
        let _ = app.emit(events::ISLAND_EDGE, EdgeState { active });
    }
    let handle = app.clone();
    // The window belongs to the UI thread; placing it there lets a DPI change
    // settle before the rectangle is verified.
    let scheduled = app.run_on_main_thread(move || {
        rice_win::window::place_topmost(hwnd, placement.window);
        if changed {
            refresh_occlusion(&handle);
            system::emit_wallpaper(&handle);
        }
    });
    if let Err(error) = scheduled {
        log::warn!("island: could not place the window: {error}");
    }
}

/// `island_configure`: the frontend's window size in CSS px.
pub fn configure(app: &AppHandle, width: f64, height: f64) -> BackendResult<()> {
    let valid = |v: f64| v.is_finite() && (1.0..=10_000.0).contains(&v);
    if !valid(width) || !valid(height) {
        return Err(BackendError::invalid_args(format!("window size {width} × {height}")));
    }
    let state = app.state::<AppState>();
    {
        let mut inner = state.island.inner.locked();
        let size = Size { width, height };
        if inner.size == size && inner.placement.is_some() {
            return Ok(());
        }
        inner.size = size;
    }
    reposition(app);
    Ok(())
}

/// `island_set_hit_regions`: clickable rectangles (CSS px, window-relative).
pub fn set_regions(app: &AppHandle, rects: Vec<Rect>) {
    let rects = rects.into_iter().filter(|r| [r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite())).collect();
    app.state::<AppState>().island.inner.locked().regions = rects;
}

/// `island_set_expanded`: expanded panels take the keyboard; compact islands never do.
pub fn set_expanded(app: &AppHandle, expanded: bool) -> BackendResult<()> {
    let window = window(app)?;
    let state = app.state::<AppState>();
    let (hwnd, previous) = {
        let mut inner = state.island.inner.locked();
        if inner.expanded == expanded {
            return Ok(());
        }
        inner.expanded = expanded;
        let previous = if expanded {
            inner.previous_foreground = rice_win::window::foreground();
            0
        } else {
            std::mem::take(&mut inner.previous_foreground)
        };
        (inner.hwnd, previous)
    };
    if expanded {
        window.set_focusable(true).map_err(BackendError::internal)?;
        window.set_focus().map_err(BackendError::internal)?;
        // The power menu in the panel shows whether updates are waiting.
        crate::power::check_updates(app);
    } else {
        // Hand the keyboard back to the app the user came from.
        if rice_win::window::foreground() == hwnd && previous != 0 && previous != hwnd {
            rice_win::window::set_foreground(previous);
        }
        window.set_focusable(false).map_err(BackendError::internal)?;
    }
    Ok(())
}

/// The island window was deactivated. If the panel is open and the
/// keyboard really went elsewhere, the frontend collapses it.
pub fn on_blur(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(BLUR_GRACE).await;
        let state = handle.state::<AppState>();
        let (expanded, hwnd) = {
            let inner = state.island.inner.locked();
            (inner.expanded, inner.hwnd)
        };
        if expanded && rice_win::window::foreground() != hwnd {
            let _ = handle.emit(events::ISLAND_FOCUS_LOST, ());
        }
    });
}

/// Computes the overlap state of the island zone right now.
pub fn compute_occlusion(app: &AppHandle) -> Occlusion {
    let state = app.state::<AppState>();
    let Some(p) = state.island.placement() else {
        return Occlusion::default();
    };
    let offset_top = state.settings.locked().offset_top;
    let zone = geometry::island_zone(p.monitor, p.work, p.scale, offset_top);
    rice_win::occlusion::query(zone, p.monitor, p.monitor_handle)
}

/// Recomputes the overlap state; sends `island://occlusion` only on change.
pub fn refresh_occlusion(app: &AppHandle) -> Occlusion {
    let occlusion = compute_occlusion(app);
    let changed = {
        let state = app.state::<AppState>();
        let mut inner = state.island.inner.locked();
        std::mem::replace(&mut inner.occlusion, occlusion) != occlusion
    };
    if changed {
        let _ = app.emit(events::ISLAND_OCCLUSION, occlusion);
    }
    occlusion
}

fn spawn_cursor_thread(app: AppHandle) -> std::io::Result<()> {
    thread::Builder::new().name("rice-cursor".into()).spawn(move || {
        let start = Instant::now();
        loop {
            thread::sleep(TICK);
            let now_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
            tick(&app, now_ms);
        }
    })?;
    Ok(())
}

/// One cursor sample: switch click-through only on changes, feed the edge detector.
fn tick(app: &AppHandle, now_ms: u64) {
    // Fails on the secure desktop (locked screen); nothing to do then.
    let Some(cursor) = rice_win::window::cursor_pos() else {
        return;
    };
    let state = app.state::<AppState>();
    let (interactive, edge) = {
        let mut guard = state.island.inner.locked();
        let inner = &mut *guard;
        let Some(p) = inner.placement else {
            return;
        };
        let origin = PhysPoint { x: p.window.x, y: p.window.y };
        let inside = geometry::hits(cursor, origin, p.scale, &inner.regions);
        // A held button keeps a drag (slider) alive past the region's edge.
        let wanted = inside || (inner.interactive && rice_win::window::mouse_button_down());
        let interactive = (wanted != inner.interactive).then(|| {
            inner.interactive = wanted;
            wanted
        });
        let edge = inner.edge.update(&EdgeInput {
            now_ms,
            cursor,
            monitor: p.monitor,
            scale: p.scale,
            window_origin: origin,
            regions: &inner.regions,
        });
        (interactive, edge)
    };
    if let Some(interactive) = interactive
        && let Some(window) = app.get_webview_window(LABEL)
        && let Err(error) = window.set_ignore_cursor_events(!interactive)
    {
        log::warn!("island: click-through switch failed: {error}");
    }
    if let Some(active) = edge {
        let _ = app.emit(events::ISLAND_EDGE, EdgeState { active });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(index: usize, primary: bool) -> Monitor {
        Monitor {
            index,
            handle: index as isize + 1,
            device: format!("\\\\.\\DISPLAY{}", index + 1),
            name: format!("Monitor {index}"),
            primary,
            internal: false,
            rect: PhysRect::default(),
            work: PhysRect::default(),
            scale: 1.0,
        }
    }

    #[test]
    fn picks_the_configured_monitor_or_the_primary() {
        let monitors = [monitor(0, false), monitor(1, true), monitor(2, false)];
        assert_eq!(pick_monitor(&monitors, MonitorSelection::Primary).map(|m| m.index), Some(1));
        assert_eq!(pick_monitor(&monitors, MonitorSelection::Index(2)).map(|m| m.index), Some(2));
        // Unplugged monitor: back to the primary one.
        assert_eq!(pick_monitor(&monitors, MonitorSelection::Index(5)).map(|m| m.index), Some(1));
        assert_eq!(pick_monitor(&[monitor(0, false)], MonitorSelection::Primary).map(|m| m.index), Some(0));
        assert!(pick_monitor(&[], MonitorSelection::Primary).is_none());
    }
}
