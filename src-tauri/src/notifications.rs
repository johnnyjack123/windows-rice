//! Notifications of other apps, mirrored into the island, and the policy for
//! Windows' own banners (docs/BACKEND.md §4 "Mitteilungen").

use crate::banners;
use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{AppNotification, NotificationPermission, NotificationsState, events};
use rice_core::media::app_name_fallback;
use rice_core::notifications::{diff_ids, launch_candidates, split_texts};
use rice_core::time::{iso_local, now_iso};
use rice_core::{BackendError, BackendResult};
use rice_services::imaging::{bytes_to_png_data_url, rgba_to_png_data_url};
use rice_win::notifications::{Access, Listener, Toast};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const DEBOUNCE: Duration = Duration::from_millis(100);
/// Polling interval where Windows does not deliver `NotificationChanged`.
const POLL: Duration = Duration::from_secs(2);
/// Safety net while the event works; also notices revoked access.
const SAFETY_POLL: Duration = Duration::from_secs(10);
/// App icons are delivered at this size (docs/BACKEND.md §4).
const ICON_SIZE: u32 = 48;

/// Notification service state.
#[derive(Default)]
pub struct Notifications {
    /// Opened on first use from a worker thread; `None` without package identity.
    listener: OnceLock<Option<Arc<Listener>>>,
    permission: Mutex<Option<NotificationPermission>>,
    /// Last listing, newest first; `None` until the first one (the baseline,
    /// which is never announced as new notifications).
    items: Mutex<Option<Vec<AppNotification>>>,
    /// Display name and icon data URL per AUMID, resolved once.
    apps: Mutex<HashMap<String, AppMeta>>,
    /// Serialises refreshes.
    refreshing: Mutex<()>,
    scheduled: AtomicBool,
    /// A `NotificationChanged` event arrived since the last poll.
    event_seen: AtomicBool,
    /// Reading from Windows failed last time (logged once per streak).
    failing: AtomicBool,
}

#[derive(Clone)]
struct AppMeta {
    name: String,
    icon: Option<String>,
}

/// Opens the listener, brings back banners a crash left hidden and starts
/// watching. Never fails: without identity everything reports `unsupported`.
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Leftovers of a crash first (docs/BACKEND.md: "bei jedem Start").
        banners::restore();
        let Some(listener) = listener(&handle) else {
            *handle.state::<AppState>().notifications.permission.locked() = Some(NotificationPermission::Unsupported);
            log::info!("notifications: no package identity; register the sparse package (README) to mirror notifications");
            return;
        };
        let events = handle.clone();
        let interval = match listener.watch(move || {
            events.state::<AppState>().notifications.event_seen.store(true, Ordering::SeqCst);
            schedule(&events);
        }) {
            Ok(_) => SAFETY_POLL,
            Err(error) => {
                log::info!("notifications: no change events ({error}); polling every 2 s");
                POLL
            }
        };
        refresh(&handle);
        poll(handle, interval);
    });
}

fn permission_of(access: Access) -> NotificationPermission {
    match access {
        Access::Granted => NotificationPermission::Granted,
        Access::Denied => NotificationPermission::Denied,
        Access::Unknown => NotificationPermission::Unknown,
    }
}

/// The listener (opened on first use). Blocking; call from worker threads only:
/// the listener lives in the MTA, the UI thread is an STA.
fn listener(app: &AppHandle) -> Option<Arc<Listener>> {
    app.state::<AppState>().notifications.listener.get_or_init(|| Listener::open().map(Arc::new)).clone()
}

/// Logs a failed Windows call once until a call succeeds again.
fn report(service: &Notifications, result: Result<(), String>) {
    match result {
        Ok(()) => service.failing.store(false, Ordering::SeqCst),
        Err(error) => {
            if !service.failing.swap(true, Ordering::SeqCst) {
                log::warn!("notifications: {error}");
            }
        }
    }
}

/// Refreshes once per burst of change events.
pub fn schedule(app: &AppHandle) {
    if app.state::<AppState>().notifications.scheduled.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DEBOUNCE).await;
        handle.state::<AppState>().notifications.scheduled.store(false, Ordering::SeqCst);
        let refresh_handle = handle.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || refresh(&refresh_handle)).await;
    });
}

/// Polls forever; drops to the short interval once a poll finds a change
/// no event announced.
fn poll(app: AppHandle, mut interval: Duration) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            let handle = app.clone();
            let changed = tauri::async_runtime::spawn_blocking(move || refresh(&handle)).await.unwrap_or(false);
            let event_seen = app.state::<AppState>().notifications.event_seen.swap(false, Ordering::SeqCst);
            if interval != POLL && changed && !event_seen {
                log::info!("notifications: change events do not arrive; polling every 2 s");
                interval = POLL;
            }
        }
    });
}

/// Re-reads access and the notification centre, applies the banner policy
/// and announces what changed. Returns whether notifications came or went.
/// Blocking.
fn refresh(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let service = &state.notifications;
    let Some(listener) = listener(app) else {
        return false;
    };
    let _guard = service.refreshing.locked();
    // An unreadable state counts as "not granted": banners then stay with Windows.
    let permission = match listener.access() {
        Ok(access) => permission_of(access),
        Err(error) => {
            report(service, Err(format!("access state unreadable ({error})")));
            NotificationPermission::Unknown
        }
    };
    let previous_permission = service.permission.locked().replace(permission);
    if previous_permission != Some(permission) {
        log::info!("notifications: access {permission:?}");
    }
    apply_banner_policy(app, permission);

    if permission != NotificationPermission::Granted {
        service.items.locked().take();
        if previous_permission != Some(permission) {
            let _ = app.emit(events::NOTIFICATIONS_STATE, NotificationsState { permission, items: Vec::new() });
        }
        return false;
    }
    let toasts = match listener.toasts() {
        Ok(toasts) => {
            report(service, Ok(()));
            toasts
        }
        Err(error) => {
            report(service, Err(format!("listing failed ({error})")));
            return false;
        }
    };
    // Newest first, like the notification list.
    let current: Vec<AppNotification> = toasts.iter().rev().map(|t| to_contract(service, t)).collect();
    let previous = service.items.locked().replace(current.clone());
    let Some(previous) = previous else {
        // Baseline after start or newly granted access.
        let _ = app.emit(events::NOTIFICATIONS_STATE, NotificationsState { permission, items: current });
        return false;
    };
    let ids = |list: &[AppNotification]| list.iter().map(|n| n.id.clone()).collect::<Vec<_>>();
    let (before, now) = (ids(&previous), ids(&current));
    let (added, removed) = diff_ids(&before, &now);
    for id in &removed {
        let _ = app.emit(events::NOTIFICATIONS_REMOVED, json!({ "id": id }));
    }
    // Oldest first, so the newest ends up on top of the list.
    for id in added.iter().rev() {
        if let Some(item) = current.iter().find(|n| n.id == *id) {
            let _ = app.emit(events::NOTIFICATIONS_POSTED, item);
        }
    }
    !added.is_empty() || !removed.is_empty()
}

/// Windows' banners are hidden exactly while rice can show notifications
/// itself: access granted and `notifications.hideWindowsBanners` on.
/// Otherwise they come back at once (docs/BACKEND.md: Sicherheitsnetz).
fn apply_banner_policy(app: &AppHandle, permission: NotificationPermission) {
    let wanted = permission == NotificationPermission::Granted && app.state::<AppState>().settings.locked().hide_windows_banners;
    if wanted && !banners::is_active() {
        banners::suppress_all();
    } else if !wanted && banners::is_active() {
        banners::restore();
    }
}

fn to_contract(service: &Notifications, toast: &Toast) -> AppNotification {
    let app = app_meta(service, toast);
    let (title, body) = split_texts(&toast.texts);
    AppNotification {
        id: toast.id.to_string(),
        app_id: toast.aumid.clone(),
        // A toast without text still needs a headline.
        title: if title.is_empty() { app.name.clone() } else { title },
        app_name: app.name,
        icon_url: app.icon,
        body,
        time: toast.created_unix_ms.map_or_else(now_iso, iso_local),
    }
}

/// Name and icon of the sending app (its logo, else its Start menu icon),
/// resolved on its first notification, which also hides its banners if the
/// app is new to Windows' settings.
fn app_meta(service: &Notifications, toast: &Toast) -> AppMeta {
    if let Some(known) = service.apps.locked().get(&toast.aumid) {
        return known.clone();
    }
    let name = toast
        .app_name
        .clone()
        .or_else(|| rice_win::media::app_display_name(&toast.aumid))
        .unwrap_or_else(|| app_name_fallback(&toast.aumid));
    let icon = toast
        .logo(ICON_SIZE as f32)
        .and_then(|bytes| bytes_to_png_data_url(&bytes, ICON_SIZE, ICON_SIZE).ok())
        .or_else(|| {
            let (width, height, pixels) = rice_win::processes::app_icon_rgba(&toast.aumid, ICON_SIZE)?;
            rgba_to_png_data_url(width, height, pixels).ok()
        });
    let meta = AppMeta { name, icon };
    service.apps.locked().insert(toast.aumid.clone(), meta.clone());
    banners::suppress_app(&toast.aumid);
    meta
}

/// Follows a changed `notifications.hideWindowsBanners`.
pub fn on_setting_changed(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let permission = handle.state::<AppState>().notifications.permission.locked().unwrap_or(NotificationPermission::Unsupported);
        apply_banner_policy(&handle, permission);
    });
}

/// `notifications_get`.
pub async fn get(app: &AppHandle) -> BackendResult<NotificationsState> {
    let handle = app.clone();
    blocking(move || {
        if listener(&handle).is_none() {
            return Ok(NotificationsState { permission: NotificationPermission::Unsupported, items: Vec::new() });
        }
        let service = &handle.state::<AppState>().notifications;
        // Asked before the first listing (or without access): read it now.
        if service.items.locked().is_none() {
            refresh(&handle);
        }
        Ok(NotificationsState {
            permission: service.permission.locked().unwrap_or(NotificationPermission::Unknown),
            items: service.items.locked().clone().unwrap_or_default(),
        })
    })
    .await
}

/// `notifications_request_access`: Windows' consent prompt (UI thread).
pub async fn request_access(app: &AppHandle) -> BackendResult<NotificationPermission> {
    if !rice_win::notifications::has_identity() {
        return Ok(NotificationPermission::Unsupported);
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let answer = sender.clone();
        if let Err(error) = rice_win::notifications::request_access(move |access| {
            let _ = answer.send(access);
        }) {
            let _ = sender.send(Err(error));
        }
    })
    .map_err(BackendError::internal)?;
    let handle = app.clone();
    blocking(move || {
        // Waits for the user's answer in the prompt.
        let access = receiver.recv().map_err(BackendError::internal)?.map_err(BackendError::internal)?;
        // Announces the new state and starts or stops hiding banners.
        refresh(&handle);
        Ok(permission_of(access))
    })
    .await
}

/// `notifications_dismiss`: removes the notification here and in Windows.
pub async fn dismiss(app: &AppHandle, id: String) -> BackendResult<()> {
    let number: u32 = id.parse().map_err(|_| BackendError::not_found(format!("no notification {id}")))?;
    let handle = app.clone();
    blocking(move || {
        let listener = listener(&handle).ok_or_else(|| BackendError::not_supported("notification access needs the rice package"))?;
        listener.remove(number).map_err(BackendError::internal)?;
        if let Some(items) = handle.state::<AppState>().notifications.items.locked().as_mut() {
            items.retain(|n| n.id != id);
        }
        let _ = handle.emit(events::NOTIFICATIONS_REMOVED, json!({ "id": id }));
        Ok(())
    })
    .await
}

/// `notifications_open`: starts (or brings forward) the sending app and
/// removes the notification, like a click on the Windows banner. Windows
/// offers no way to replay the notification's own click action (say, the
/// chat it points to), so the app opens where it last was.
pub async fn open(app: &AppHandle, id: String) -> BackendResult<()> {
    let handle = app.clone();
    blocking(move || {
        let service = &handle.state::<AppState>().notifications;
        let aumid = service
            .items
            .locked()
            .as_ref()
            .and_then(|items| items.iter().find(|n| n.id == id).map(|n| n.app_id.clone()))
            .ok_or_else(|| BackendError::not_found(format!("no notification {id}")))?;
        let opened = launch_candidates(&aumid).iter().any(|candidate| match rice_win::shell::open_app(candidate) {
            Ok(()) => true,
            Err(error) => {
                log::debug!("notifications: cannot start {candidate} ({error})");
                false
            }
        });
        if !opened {
            return Err(BackendError::not_supported(format!("Windows cannot start {aumid}")));
        }
        // Like Windows: a clicked notification leaves the notification centre.
        if let (Some(listener), Ok(number)) = (listener(&handle), id.parse::<u32>()) {
            let _ = listener.remove(number);
        }
        if let Some(items) = service.items.locked().as_mut() {
            items.retain(|n| n.id != id);
        }
        let _ = handle.emit(events::NOTIFICATIONS_REMOVED, json!({ "id": id }));
        Ok(())
    })
    .await
}

/// `notifications_clear`: empties Windows' notification centre.
pub async fn clear(app: &AppHandle) -> BackendResult<()> {
    let handle = app.clone();
    blocking(move || {
        let Some(listener) = listener(&handle) else {
            return Ok(());
        };
        listener.clear().map_err(BackendError::internal)?;
        let removed = handle.state::<AppState>().notifications.items.locked().as_mut().map(std::mem::take).unwrap_or_default();
        for item in removed {
            let _ = handle.emit(events::NOTIFICATIONS_REMOVED, json!({ "id": item.id }));
        }
        Ok(())
    })
    .await
}

/// `rice.exe --check-notifications`: what the listener sees, without any
/// window and without asking for access (development aid).
pub fn print_diagnosis() {
    println!("package identity: {}", rice_win::notifications::has_identity());
    let Some(listener) = Listener::open() else {
        println!("listener: unavailable");
        return;
    };
    match listener.access() {
        Ok(access) => println!("access: {access:?}"),
        Err(error) => println!("access: unreadable ({error})"),
    }
    match listener.watch(|| {}) {
        Ok(_) => println!("change events: available"),
        Err(error) => println!("change events: unavailable ({error})"),
    }
    match listener.toasts() {
        Ok(toasts) => println!("notifications in the notification centre: {}", toasts.len()),
        Err(error) => println!("listing: failed ({error})"),
    }
}
