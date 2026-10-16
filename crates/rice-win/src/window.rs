//! Window, cursor and foreground helpers. Window handles cross threads as
//! `isize` because `HWND` is not `Send`.

use rice_core::geometry::{PhysPoint, PhysRect};
use std::mem;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindowRect, HWND_TOPMOST, IsWindow,
    RegisterClassW, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SetForegroundWindow, SetWindowPos, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::w;

/// Creates the invisible window that owns the island and break windows.
/// Owned windows stay out of the taskbar and Alt+Tab, which `skip_taskbar`
/// alone does not achieve. Call on the thread that runs the UI message loop.
pub fn create_owner() -> windows::core::Result<isize> {
    // SAFETY: class registration and creation of a never-shown window on the
    // calling (UI) thread, whose message loop dispatches to `owner_proc`.
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = w!("rice-owner");
        let wc = WNDCLASSW { lpfnWndProc: Some(owner_proc), hInstance: instance.into(), lpszClassName: class, ..Default::default() };
        if RegisterClassW(&wc) == 0 {
            return Err(windows::core::Error::from_thread());
        }
        let window = CreateWindowExW(WS_EX_TOOLWINDOW, class, w!("rice"), WS_POPUP, 0, 0, 0, 0, None, None, Some(instance.into()), None)?;
        Ok(raw(window))
    }
}

unsafe extern "system" fn owner_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: default handling only.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// Rebuilds a window handle from its integer form.
pub fn hwnd(raw: isize) -> HWND {
    HWND(raw as *mut _)
}

/// Integer form of a window handle.
pub fn raw(hwnd: HWND) -> isize {
    hwnd.0 as isize
}

/// Current cursor position on the virtual screen (physical px).
pub fn cursor_pos() -> Option<PhysPoint> {
    let mut point = POINT::default();
    // SAFETY: out-pointer to a local. Fails on the secure desktop (locked).
    unsafe { GetCursorPos(&mut point) }.ok()?;
    Some(PhysPoint { x: point.x, y: point.y })
}

/// Whether any mouse button is held (keeps a drag alive across the edge of a hit region).
pub fn mouse_button_down() -> bool {
    // SAFETY: no pointers involved.
    unsafe {
        [VK_LBUTTON, VK_RBUTTON, VK_MBUTTON]
            .iter()
            .any(|key| (GetAsyncKeyState(i32::from(key.0)) as u16) & 0x8000 != 0)
    }
}

/// The foreground window, 0 if none.
pub fn foreground() -> isize {
    // SAFETY: no arguments.
    raw(unsafe { GetForegroundWindow() })
}

/// Hands the foreground to a window (allowed while this process owns it).
pub fn set_foreground(window: isize) -> bool {
    // SAFETY: Windows validates the handle.
    unsafe { IsWindow(Some(hwnd(window))).as_bool() && SetForegroundWindow(hwnd(window)).as_bool() }
}

/// Outer rectangle of a window.
pub fn window_rect(window: isize) -> Option<PhysRect> {
    let mut r = RECT::default();
    // SAFETY: out-pointer to a local.
    unsafe { GetWindowRect(hwnd(window), &mut r) }.ok()?;
    Some(PhysRect::from_edges(r.left, r.top, r.right, r.bottom))
}

/// Visible frame of a window without the invisible resize borders.
pub fn frame_rect(window: isize) -> Option<PhysRect> {
    let mut r = RECT::default();
    // SAFETY: DWMWA_EXTENDED_FRAME_BOUNDS writes one RECT.
    let ok = unsafe {
        DwmGetWindowAttribute(hwnd(window), DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut RECT as *mut _, mem::size_of::<RECT>() as u32)
    };
    match ok {
        Ok(()) => Some(PhysRect::from_edges(r.left, r.top, r.right, r.bottom)),
        Err(_) => window_rect(window),
    }
}

/// Whether DWM hides the window (other virtual desktop, suspended UWP app …).
pub fn is_cloaked(window: isize) -> bool {
    let mut cloaked = 0u32;
    // SAFETY: DWMWA_CLOAKED writes one DWORD.
    let ok = unsafe {
        DwmGetWindowAttribute(hwnd(window), DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, mem::size_of::<u32>() as u32)
    };
    ok.is_ok() && cloaked != 0
}

/// Window class name.
pub fn class_name(window: isize) -> String {
    let mut buffer = [0u16; 128];
    // SAFETY: Windows writes at most buffer.len() characters.
    let len = unsafe { GetClassNameW(hwnd(window), &mut buffer) };
    String::from_utf16_lossy(&buffer[..usize::try_from(len).unwrap_or(0)])
}

/// Moves and sizes a topmost window without activating it. Moving onto a
/// monitor with another DPI makes Windows resize it once more, so the
/// rectangle is checked and applied a second time if needed.
///
/// Call this on the thread that owns the window.
pub fn place_topmost(window: isize, rect: PhysRect) {
    for _ in 0..2 {
        // SAFETY: Windows validates the handle.
        let _ = unsafe {
            SetWindowPos(hwnd(window), Some(HWND_TOPMOST), rect.x, rect.y, rect.width, rect.height, SWP_NOACTIVATE | SWP_NOOWNERZORDER)
        };
        if window_rect(window) == Some(rect) {
            return;
        }
    }
    log::warn!("window: could not place {window:#x} at {rect:?}; got {:?}", window_rect(window));
}
