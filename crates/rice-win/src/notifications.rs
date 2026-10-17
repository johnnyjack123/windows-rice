//! Notifications of other apps via `UserNotificationListener`
//! (docs/BACKEND.md §4 "Mitteilungen").
//!
//! The listener needs package identity, which rice gets from the sparse
//! package in `src-tauri/package`. Without it [`Listener::open`] returns
//! `None` and nothing else is touched.
//!
//! Async WinRT calls are awaited with `join()`, which blocks: call the
//! listing functions from worker threads.

use rice_core::time::windows_ticks_to_unix_ms;
use windows::ApplicationModel::AppInfo;
use windows::Foundation::{Size, TypedEventHandler};
use windows::Storage::Streams::IRandomAccessStreamReference;
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus as Status};
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds, UserNotification};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS};
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows_core::Interface;
use windows_future::IAsyncOperation;

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

/// The notification listener of this process (agile, usable from any thread).
pub struct Listener(UserNotificationListener);

/// A consent prompt that is still open.
pub struct AccessRequest(IAsyncOperation<Status>);

impl Listener {
    /// The listener, or `None` without package identity.
    pub fn open() -> Option<Self> {
        if !has_identity() {
            return None;
        }
        UserNotificationListener::Current().ok().map(Self)
    }

    /// The current access state (fast, no prompt).
    pub fn access(&self) -> Access {
        match self.0.GetAccessStatus() {
            Ok(Status::Allowed) => Access::Granted,
            Ok(Status::Denied) => Access::Denied,
            _ => Access::Unknown,
        }
    }

    /// Opens Windows' consent prompt. Call on the UI thread, then wait for the
    /// answer with [`AccessRequest::wait`] on another one.
    pub fn request_access(&self) -> windows::core::Result<AccessRequest> {
        self.0.RequestAccessAsync().map(AccessRequest)
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

impl AccessRequest {
    /// Waits for the user's answer (blocks).
    pub fn wait(self) -> windows::core::Result<Access> {
        Ok(match self.0.join()? {
            Status::Allowed => Access::Granted,
            Status::Denied => Access::Denied,
            _ => Access::Unknown,
        })
    }
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
