//! System notifications on a dedicated thread: a hidden top-level window
//! receives broadcast messages (settings, wallpaper, displays), WinEvent hooks
//! report window movement, and `UISettings` reports accent changes.
//!
//! Bursts are coalesced with window timers so the handler sees one event per
//! change, not dozens.

use crate::com::ComGuard;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;
use windows::Foundation::TypedEventHandler;
use windows::UI::ViewManagement::UISettings;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook};
use windows::Win32::UI::WindowsAndMessaging::{
    CHILDID_SELF, CreateWindowExW, DefWindowProcW, DispatchMessageW, EVENT_OBJECT_CLOAKED, EVENT_OBJECT_HIDE,
    EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW, EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, GA_ROOT, GetAncestor, GetMessageW, KillTimer, MSG, OBJID_WINDOW,
    PostMessageW, RegisterClassW, SPI_SETDESKWALLPAPER, SPI_SETWORKAREA, SetTimer, TranslateMessage, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_APP, WM_DISPLAYCHANGE, WM_SETTINGCHANGE, WM_TIMER, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};
use windows::core::w;

/// What changed, coalesced over a short period.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SystemChange {
    /// Dark mode, accent colour, animations or transparency may have changed.
    pub appearance: bool,
    pub wallpaper: bool,
    /// A taskbar moved, appeared or auto-hides now.
    pub work_area: bool,
    /// Monitors were added, removed or changed resolution.
    pub displays: bool,
}

/// Events delivered to the handler, always on the system-events thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemEvent {
    Changed(SystemChange),
    /// A top-level window moved, appeared, vanished or came to the front
    /// (at most every 50 ms while it keeps happening).
    WindowsMoved,
}

const TIMER_SETTINGS: usize = 1;
const TIMER_WINDOWS: usize = 2;
const WM_APP_COLORS: u32 = WM_APP + 1;
const SETTINGS_DELAY_MS: u32 = 150;
const DISPLAY_DELAY_MS: u32 = 400;
const WINDOWS_DELAY_MS: u32 = 50;

#[derive(Default)]
struct State {
    hwnd: isize,
    handler: Option<Rc<dyn Fn(SystemEvent)>>,
    pending: SystemChange,
    windows_pending: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Starts the system-events thread. The handler must return quickly; heavy
/// work belongs on another thread.
pub fn spawn(handler: impl Fn(SystemEvent) + Send + 'static) -> std::io::Result<()> {
    thread::Builder::new().name("rice-system-events".into()).spawn(move || {
        if let Err(error) = run(Rc::new(handler)) {
            log::error!("system events: {error}");
        }
    })?;
    Ok(())
}

fn run(handler: Rc<dyn Fn(SystemEvent)>) -> windows::core::Result<()> {
    let _com = ComGuard::sta();
    // SAFETY: standard window class registration and creation on this thread;
    // the window lives as long as the thread's message loop.
    let hwnd = unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = w!("rice-system-events");
        let wc = WNDCLASSW { lpfnWndProc: Some(window_proc), hInstance: instance.into(), lpszClassName: class, ..Default::default() };
        if RegisterClassW(&wc) == 0 {
            return Err(windows::core::Error::from_thread());
        }
        // A hidden top-level window: message-only windows miss broadcasts.
        CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, class, w!("rice"), WS_POPUP, 0, 0, 0, 0, None, None, Some(instance.into()), None)?
    };
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.hwnd = hwnd.0 as isize;
        s.handler = Some(handler);
    });

    let ranges = [
        (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
        (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
        (EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE),
        (EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE),
        (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
    ];
    for (min, max) in ranges {
        // SAFETY: out-of-context hooks call back on this thread while it pumps messages.
        let hook = unsafe { SetWinEventHook(min, max, None, Some(on_win_event), 0, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS) };
        if hook.is_invalid() {
            log::warn!("system events: WinEvent hook {min:#x}..{max:#x} failed");
        }
    }

    // Accent colour changes do not always come with WM_SETTINGCHANGE.
    let raw = hwnd.0 as isize;
    let ui_settings = UISettings::new().ok();
    if let Some(settings) = &ui_settings {
        let handler = TypedEventHandler::new(move |_, _| {
            // SAFETY: posting to our own window; fails harmlessly once it is gone.
            let _ = unsafe { PostMessageW(Some(HWND(raw as *mut _)), WM_APP_COLORS, WPARAM(0), LPARAM(0)) };
            Ok(())
        });
        if let Err(error) = settings.ColorValuesChanged(&handler) {
            log::warn!("system events: ColorValuesChanged: {error}");
        }
    }

    let mut msg = MSG::default();
    // SAFETY: standard message loop for the window created above.
    unsafe {
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    drop(ui_settings);
    Ok(())
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SETTINGCHANGE => {
            let action = wparam.0 as u32;
            mark(SETTINGS_DELAY_MS, |c| {
                c.appearance = true;
                c.wallpaper |= action == SPI_SETDESKWALLPAPER.0;
                c.work_area |= action == SPI_SETWORKAREA.0;
            });
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            mark(DISPLAY_DELAY_MS, |c| {
                c.displays = true;
                c.work_area = true;
                c.wallpaper = true;
            });
            LRESULT(0)
        }
        WM_APP_COLORS => {
            mark(SETTINGS_DELAY_MS, |c| c.appearance = true);
            LRESULT(0)
        }
        WM_TIMER => {
            // SAFETY: our own timers on our own window.
            let _ = unsafe { KillTimer(Some(hwnd), wparam.0) };
            let event = STATE.with(|s| {
                let mut s = s.borrow_mut();
                match wparam.0 {
                    TIMER_SETTINGS => Some(SystemEvent::Changed(std::mem::take(&mut s.pending))),
                    TIMER_WINDOWS => {
                        s.windows_pending = false;
                        Some(SystemEvent::WindowsMoved)
                    }
                    _ => None,
                }
            });
            if let Some(event) = event {
                dispatch(event);
            }
            LRESULT(0)
        }
        // SAFETY: default handling for everything else.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if hwnd.is_invalid() || id_object != OBJID_WINDOW.0 || id_child != CHILDID_SELF as i32 {
        return;
    }
    // SAFETY: read-only query; Windows validates the handle.
    if event == EVENT_OBJECT_LOCATIONCHANGE && unsafe { GetAncestor(hwnd, GA_ROOT) } != hwnd {
        return;
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if !s.windows_pending {
            s.windows_pending = true;
            // SAFETY: timer on our own window.
            unsafe { SetTimer(Some(HWND(s.hwnd as *mut _)), TIMER_WINDOWS, WINDOWS_DELAY_MS, None) };
        }
    });
}

/// Records a change and (re)starts the settings timer: the handler runs once
/// the burst has been quiet for `delay_ms`.
fn mark(delay_ms: u32, update: impl FnOnce(&mut SystemChange)) {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        update(&mut s.pending);
        // SAFETY: timer on our own window; re-arming resets it.
        unsafe { SetTimer(Some(HWND(s.hwnd as *mut _)), TIMER_SETTINGS, delay_ms, None) };
    });
}

fn dispatch(event: SystemEvent) {
    // Clone the handler out so it may run without holding the state borrow.
    let handler = STATE.with(|s| s.borrow().handler.clone());
    if let Some(handler) = handler {
        handler(event);
    }
}
