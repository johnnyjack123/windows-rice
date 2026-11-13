//! Notifications of other apps via `UserNotificationListener`
//! (docs/BACKEND.md §4 "Mitteilungen").
//!
//! The listener needs package identity, which rice gets from the sparse
//! package in `src-tauri/package`. Without it [`Listener::open`] returns
//! `None` and nothing else is touched.
//!
//! The listener belongs to the COM apartment that creates it. [`Listener::open`]
//! creates it in the multithreaded apartment, which rice's worker threads
//! share, so it may be used from any of them, never from the UI thread (an
//! STA). Only the consent prompt runs on the UI thread, with its own
//! instance ([`request_access`]). Async WinRT calls are awaited with
//! `join()`, which blocks.

use crate::com::ComGuard;
use rice_core::time::windows_ticks_to_unix_ms;
use std::sync::Once;
use windows::ApplicationModel::AppInfo;
use windows::Foundation::{Size, TypedEventHandler};
use windows::Storage::Streams::IRandomAccessStreamReference;
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus as Status};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Com::CoIncrementMTAUsage;
use windows_core::Interface;

/// Whether rice may read other apps' notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Granted,
    Denied,
    /// The user has not decided yet.
    Unknown,
}

/// Whether this process runs with package identity.
pub fn has_identity() -> bool {
    let mut len = 0u32;
    // SAFETY: length query without a buffer.
    let status = unsafe { GetCurrentPackageFullName(&mut len, None) };
    status == ERROR_INSUFFICIENT_BUFFER || status == ERROR_SUCCESS
}

/// A toast from the notification centre.
pub struct Toast {
    pub id: u32,
    /// AUMID of the sending app (empty if Windows does not say).
    pub aumid: String,
    /// Display name, if Windows knows the app.
    pub app_name: Option<String>,
    /// Texts of the generic toast binding, in order.
    pub texts: Vec<String>,
    pub created_unix_ms: Option<i64>,
    info: Option<AppInfo>,
}

impl Toast {
    /// The app's logo (any image format) at about `size` px, if the app has one.
    pub fn logo(&self, size: f32) -> Option<Vec<u8>> {
        let reference = self.info.as_ref()?.DisplayInfo().ok()?.GetLogo(Size { Width: size, Height: size }).ok()?;
        let reference: IRandomAccessStreamReference = reference.cast().ok()?;
        crate::media::read_all(&reference).ok().filter(|bytes| !bytes.is_empty())
    }
}

/// The notification listener of this process. It lives in the MTA: use it
/// from worker threads, never from the UI thread.
pub struct Listener(UserNotificationListener);

impl Listener {
    /// The listener, or `None` without package identity. Created on a
    /// short-lived MTA thread, whatever the caller's apartment is.
    pub fn open() -> Option<Self> {
        if !has_identity() {
            return None;
        }
        let created = std::thread::spawn(|| {
            let _com = ComGuard::mta();
            keep_mta_alive();
            UserNotificationListener::Current().map(Self)
        })
        .join();
        match created {
            Ok(Ok(listener)) => Some(listener),
            Ok(Err(error)) => {
                log::warn!("notifications: listener unavailable ({error})");
                None
            }
            Err(_) => None,
        }
    }

    /// The current access state (fast, no prompt).
    pub fn access(&self) -> windows::core::Result<Access> {
        self.0.GetAccessStatus().map(access_of)
    }

    /// All toasts in the notification centre, oldest first.
    pub fn toasts(&self) -> windows::core::Result<Vec<Toast>> {
        let list = self.0.GetNotificationsAsync(NotificationKinds::Toast)?.join()?;
        let count = list.Size()?;
        let mut toasts: Vec<Toast> = (0..count).filter_map(|i| list.GetAt(i).ok()).filter_map(|n| toast(&n)).collect();
        toasts.sort_by_key(|t| (t.created_unix_ms.unwrap_or(0), t.id));
        Ok(toasts)
    }

    /// Removes one toast from the notification centre.
    pub fn remove(&self, id: u32) -> windows::core::Result<()> {
        self.0.RemoveNotification(id)
    }

    /// Removes every toast from the notification centre.
    pub fn clear(&self) -> windows::core::Result<()> {
        self.0.ClearNotifications()
    }

    /// Calls `changed` whenever a toast arrives or goes. Fails where Windows
    /// does not offer the event to this app; poll then.
    pub fn watch(&self, changed: impl Fn() + Send + 'static) -> windows::core::Result<i64> {
        self.0.NotificationChanged(&TypedEventHandler::new(move |_, _| {
            changed();
            Ok(())
        }))
    }
}

/// Opens Windows' consent prompt and reports the answer through `done`.
/// Call on the UI thread; `done` runs later, on another thread.
pub fn request_access(done: impl FnOnce(windows::core::Result<Access>) + Send + 'static) -> windows::core::Result<()> {
    UserNotificationListener::Current()?.RequestAccessAsync()?.when(move |answer| done(answer.map(access_of)))
}

fn access_of(status: Status) -> Access {
    match status {
        Status::Allowed => Access::Granted,
        Status::Denied => Access::Denied,
        _ => Access::Unknown,
    }
}

/// Keeps the MTA alive for the whole process, so objects living in it stay
/// usable after the thread that created them has ended.
fn keep_mta_alive() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: plain call; the cookie is deliberately never released.
        if let Err(error) = unsafe { CoIncrementMTAUsage() } {
            log::warn!("notifications: cannot keep the MTA alive ({error})");
        }
    });
}

fn toast(notification: &UserNotification) -> Option<Toast> {
    let id = notification.Id().ok()?;
    let info = notification.AppInfo().ok();
    let aumid = info.as_ref().and_then(|i| i.AppUserModelId().ok()).map(|a| a.to_string()).unwrap_or_default();
    let app_name = info
        .as_ref()
        .and_then(|i| i.DisplayInfo().ok())
        .and_then(|d| d.DisplayName().ok())
        .map(|n| n.to_string())
        .filter(|n| !n.trim().is_empty());
    let created_unix_ms = notification.CreationTime().ok().and_then(|t| windows_ticks_to_unix_ms(t.UniversalTime));
    Some(Toast { id, aumid, app_name, texts: texts(notification).unwrap_or_default(), created_unix_ms, info })
}

fn texts(notification: &UserNotification) -> windows::core::Result<Vec<String>> {
    let binding = notification.Notification()?.Visual()?.GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
    let elements = binding.GetTextElements()?;
    let count = elements.Size()?;
    Ok((0..count).filter_map(|i| elements.GetAt(i).ok()).filter_map(|t| t.Text().ok()).map(|t| t.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_package_identity() {
        // Test binaries never run with identity.
        assert!(!has_identity());
        assert!(Listener::open().is_none());
    }
}
