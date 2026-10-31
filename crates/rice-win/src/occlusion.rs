//! Does another window cover the island zone, and is a full-screen app in front?

use crate::window::{class_name, foreground, frame_rect, hwnd, is_cloaked, window_rect};
use rice_core::contract::Occlusion;
use rice_core::geometry::PhysRect;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONULL, MonitorFromWindow};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetWindowLongPtrW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, WS_EX_TOOLWINDOW,
    WS_EX_TRANSPARENT,
};
use windows::core::BOOL;

/// Desktop and taskbar windows never count as covering the island.
const SHELL_CLASSES: [&str; 4] = ["Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

/// Computes the occlusion state for the island monitor.
///
/// `zone` is the island zone (see `rice_core::geometry::island_zone`),
/// `monitor` the full monitor rectangle and `monitor_handle` its `HMONITOR`.
pub fn query(zone: PhysRect, monitor: PhysRect, monitor_handle: isize) -> Occlusion {
    // SAFETY: no arguments.
    let own_pid = unsafe { GetCurrentProcessId() };
    let occluded = top_level_windows().into_iter().any(|w| covers(w, own_pid, zone));
    Occlusion { occluded, fullscreen: fullscreen(monitor, monitor_handle, own_pid) }
}

/// Who hides the island right now, for the log: the first window over the
/// zone, otherwise the full-screen app in front. Program and window class
/// only; titles can name private documents.
pub fn culprit(zone: PhysRect) -> Option<String> {
    // SAFETY: no arguments.
    let own_pid = unsafe { GetCurrentProcessId() };
    let window = top_level_windows().into_iter().find(|&w| covers(w, own_pid, zone)).or_else(|| {
        let front = foreground();
        (front != 0 && is_candidate(front, own_pid)).then_some(front)
    })?;
    let mut pid = 0u32;
    // SAFETY: read-only query into a local.
    unsafe { GetWindowThreadProcessId(hwnd(window), Some(&mut pid)) };
    let program = crate::processes::exe_path(pid)
        .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        .unwrap_or_else(|| format!("pid {pid}"));
    Some(format!("{program} ({})", class_name(window)))
}

fn top_level_windows() -> Vec<isize> {
    let mut windows: Vec<isize> = Vec::new();
    // SAFETY: the callback only pushes into the Vec passed through LPARAM,
    // which outlives the synchronous enumeration.
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut windows as *mut Vec<isize> as isize));
    }
    windows
}

unsafe extern "system" fn collect(window: HWND, data: LPARAM) -> BOOL {
    // SAFETY: `data` is the &mut Vec handed to EnumWindows above.
    let windows = unsafe { &mut *(data.0 as *mut Vec<isize>) };
    windows.push(window.0 as isize);
    BOOL(1)
}

/// A window that a user would see on top of the desktop, not ours, not the shell.
fn is_candidate(window: isize, own_pid: u32) -> bool {
    let handle = hwnd(window);
    // SAFETY: read-only queries; Windows validates the handle.
    unsafe {
        if !IsWindowVisible(handle).as_bool() || IsIconic(handle).as_bool() {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(handle, Some(&mut pid));
        if pid == own_pid {
            return false;
        }
        // Tool windows and click-through overlays (game/recording HUDs) are ignored.
        let ex = GetWindowLongPtrW(handle, GWL_EXSTYLE) as u32;
        if ex & (WS_EX_TOOLWINDOW.0 | WS_EX_TRANSPARENT.0) != 0 {
            return false;
        }
    }
    !is_cloaked(window) && !SHELL_CLASSES.contains(&class_name(window).as_str())
}

fn covers(window: isize, own_pid: u32, zone: PhysRect) -> bool {
    is_candidate(window, own_pid) && frame_rect(window).is_some_and(|r| r.intersects(&zone))
}

fn fullscreen(monitor: PhysRect, monitor_handle: isize, own_pid: u32) -> bool {
    // SAFETY: no arguments.
    let state = unsafe { SHQueryUserNotificationState() }.ok();
    if state == Some(QUNS_PRESENTATION_MODE) {
        return true;
    }
    let front = foreground();
    if front == 0 || !is_candidate(front, own_pid) {
        return false;
    }
    // SAFETY: Windows validates the handle.
    let front_monitor = unsafe { MonitorFromWindow(hwnd(front), MONITOR_DEFAULTTONULL) };
    if front_monitor.0 as isize != monitor_handle {
        return false;
    }
    matches!(state, Some(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN))
        || window_rect(front).is_some_and(|r| r.contains_rect(&monitor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowTextW;

    /// `cargo test -p rice-win prints_covering_windows -- --ignored --nocapture` (read-only):
    /// which windows hide the island on each monitor, and the full-screen verdict.
    #[test]
    #[ignore = "diagnostic: lists the windows that count as covering the island zone"]
    fn prints_covering_windows() {
        // SAFETY: no arguments.
        let own_pid = unsafe { GetCurrentProcessId() };
        for monitor in crate::monitors::list() {
            let zone = rice_core::geometry::island_zone(monitor.rect, monitor.work, monitor.scale, 0.0);
            let state = query(zone, monitor.rect, monitor.handle);
            println!("monitor {} {:?}: zone {zone:?} → {state:?}", monitor.index, monitor.name);
            for window in top_level_windows().into_iter().filter(|&w| covers(w, own_pid, zone)) {
                let mut pid = 0u32;
                let mut title = [0u16; 256];
                // SAFETY: read-only queries into locals.
                let length = unsafe {
                    GetWindowThreadProcessId(hwnd(window), Some(&mut pid));
                    GetWindowTextW(hwnd(window), &mut title)
                };
                let title = String::from_utf16_lossy(&title[..usize::try_from(length).unwrap_or(0)]);
                let exe = crate::processes::exe_path(pid).map(|p| p.display().to_string()).unwrap_or_default();
                println!("  {window:#x} class {:?} title {title:?} pid {pid} {exe}", class_name(window));
                println!("    frame {:?}", frame_rect(window));
            }
        }
    }
}
