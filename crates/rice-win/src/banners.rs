//! The per-app switch "Show notification banners" in the registry:
//! DWORD `ShowBanner` below `HKCU\…\Notifications\Settings\<AUMID>`
//! (docs/BACKEND.md §4 "Mitteilungen").
//!
//! Only `ShowBanner` is ever written, and only in keys that already exist;
//! policy and backups live in `rice-services`.

use rice_core::notifications::{AppBanner, BannerStore};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_SAM_FLAGS, RRF_RT_REG_DWORD,
    RegCloseKey, RegDeleteValueW, RegEnumKeyExW, RegEnumValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::core::{HSTRING, PCWSTR, PWSTR, w};

/// Where Windows keeps the per-app notification settings (below HKCU).
pub const SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings";

/// Path AUMIDs nest a few levels deep; anything deeper is not an app.
const MAX_DEPTH: usize = 12;
/// Longest registry key name plus NUL.
const NAME_CAPACITY: usize = 256;

/// The settings tree, normally [`SETTINGS`]; tests use a scratch key.
#[derive(Debug, Clone)]
pub struct Settings {
    root: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self::at(SETTINGS)
    }
}

impl Settings {
    /// A settings tree at `root` below HKCU.
    pub fn at(root: &str) -> Self {
        Self { root: root.trim_end_matches('\\').to_owned() }
    }

    fn full(&self, path: &str) -> String {
        format!(r"{}\{path}", self.root)
    }

    fn collect(&self, path: &str, depth: usize, apps: &mut Vec<AppBanner>) {
        let Some(key) = Key::open(&self.full(path), KEY_READ) else {
            return;
        };
        let children = key.subkeys();
        // Intermediate keys of path AUMIDs ("C:", "Tools") carry no values.
        if (key.has_values() || children.is_empty())
            && let Some(app) = read_app(&key, path)
        {
            apps.push(app);
        }
        if depth < MAX_DEPTH {
            for child in children {
                self.collect(&format!(r"{path}\{child}"), depth + 1, apps);
            }
        }
    }
}

impl BannerStore for Settings {
    /// Every app key (recursively). Keys whose `ShowBanner` is not a DWORD are
    /// left out, so rice never touches what it does not understand.
    fn apps(&self) -> Vec<AppBanner> {
        let mut apps = Vec::new();
        if let Some(root) = Key::open(&self.root, KEY_READ) {
            for name in root.subkeys() {
                self.collect(&name, 1, &mut apps);
            }
        }
        apps
    }

    fn app(&self, path: &str) -> Option<AppBanner> {
        if path.is_empty() {
            return None;
        }
        let key = Key::open(&self.full(path), KEY_READ)?;
        read_app(&key, path)
    }

    /// Never creates keys: a missing app is an error.
    fn set_show_banner(&self, path: &str, value: Option<u32>) -> Result<(), String> {
        let key = Key::open(&self.full(path), KEY_SET_VALUE | KEY_QUERY_VALUE).ok_or_else(|| format!("no settings for {path}"))?;
        // SAFETY: open key; NUL-terminated value name; data is a 4-byte DWORD.
        let status = unsafe {
            match value {
                Some(value) => RegSetValueExW(key.0, w!("ShowBanner"), None, REG_DWORD, Some(&value.to_le_bytes())),
                None => RegDeleteValueW(key.0, w!("ShowBanner")),
            }
        };
        match status {
            ERROR_SUCCESS => Ok(()),
            ERROR_FILE_NOT_FOUND if value.is_none() => Ok(()),
            error => Err(format!("ShowBanner of {path}: {}", windows::core::Error::from(error.to_hresult()).message())),
        }
    }
}

fn read_app(key: &Key, path: &str) -> Option<AppBanner> {
    let show_banner = key.dword(w!("ShowBanner")).ok()?;
    let enabled = key.dword(w!("Enabled")).ok().flatten();
    Some(AppBanner { path: path.to_owned(), enabled, show_banner })
}

/// An open registry key, closed on drop.
struct Key(HKEY);

impl Key {
    fn open(path: &str, access: REG_SAM_FLAGS) -> Option<Self> {
        let mut key = HKEY::default();
        // SAFETY: NUL-terminated path; the handle is closed in Drop.
        let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, access, &mut key) };
        (status == ERROR_SUCCESS).then_some(Self(key))
    }

    fn subkeys(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut buffer = [0u16; NAME_CAPACITY];
        for index in 0.. {
            let mut len = buffer.len() as u32;
            // SAFETY: buffer of `len` characters; optional out-params unused.
            let status = unsafe { RegEnumKeyExW(self.0, index, Some(PWSTR(buffer.as_mut_ptr())), &mut len, None, None, None, None) };
            if status != ERROR_SUCCESS {
                break;
            }
            names.push(String::from_utf16_lossy(&buffer[..len as usize]));
        }
        names
    }

    fn has_values(&self) -> bool {
        // Value names may be up to 16383 characters.
        let mut buffer = vec![0u16; 16_384];
        let mut len = buffer.len() as u32;
        // SAFETY: buffer of `len` characters; type and data are not requested.
        let status = unsafe { RegEnumValueW(self.0, 0, Some(PWSTR(buffer.as_mut_ptr())), &mut len, None, None, None, None) };
        status == ERROR_SUCCESS || status == ERROR_MORE_DATA
    }

    /// `Ok(None)` when the value is missing, `Err` when it is not a DWORD.
    fn dword(&self, name: PCWSTR) -> Result<Option<u32>, WIN32_ERROR> {
        let mut data = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: open key; out-buffer of exactly `size` bytes.
        let status = unsafe {
            RegGetValueW(self.0, PCWSTR::null(), name, RRF_RT_REG_DWORD, None, Some((&mut data as *mut u32).cast()), Some(&mut size))
        };
        match status {
            ERROR_SUCCESS => Ok(Some(data)),
            ERROR_FILE_NOT_FOUND => Ok(None),
            error => Err(error),
        }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from RegOpenKeyExW and is closed once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Registry::{KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCreateKeyExW, RegDeleteTreeW};

    /// A scratch tree below `HKCU\Software`, removed on drop.
    struct Scratch(String);

    impl Scratch {
        fn new() -> Self {
            Self(format!(r"Software\rice-test-{}", crate::credentials::new_uuid()))
        }

        fn create(&self, path: &str, values: &[(&str, Option<u32>)]) {
            let mut key = HKEY::default();
            unsafe {
                let full = HSTRING::from(format!(r"{}\{path}", self.0));
                let status = RegCreateKeyExW(HKEY_CURRENT_USER, &full, None, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None);
                assert_eq!(status, ERROR_SUCCESS);
                for (name, value) in values {
                    let name = HSTRING::from(*name);
                    let status = match value {
                        Some(v) => RegSetValueExW(key, &name, None, REG_DWORD, Some(&v.to_le_bytes())),
                        // A string where a DWORD belongs.
                        None => RegSetValueExW(key, &name, None, REG_SZ, Some(&[b'x', 0, 0, 0])),
                    };
                    assert_eq!(status, ERROR_SUCCESS);
                }
                let _ = RegCloseKey(key);
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            // Deletes the scratch key with everything below it.
            let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(&self.0)) };
        }
    }

    #[test]
    fn lists_and_switches_app_keys() {
        let scratch = Scratch::new();
        scratch.create("Discord", &[("LastNotificationAddedTime", Some(1))]);
        scratch.create("Signal", &[("ShowBanner", Some(1)), ("Enabled", Some(1))]);
        scratch.create("Off", &[("Enabled", Some(0))]);
        scratch.create("Weird", &[("ShowBanner", None)]);
        scratch.create(r"C:\Tools\app.exe", &[("LastNotificationAddedTime", Some(1))]);
        let settings = Settings::at(&scratch.0);

        let mut apps = settings.apps();
        apps.sort_by(|a, b| a.path.cmp(&b.path));
        let summary: Vec<(&str, Option<u32>, Option<u32>)> =
            apps.iter().map(|a| (a.path.as_str(), a.enabled, a.show_banner)).collect();
        assert_eq!(
            summary,
            vec![(r"C:\Tools\app.exe", None, None), ("Discord", None, None), ("Off", Some(0), None), ("Signal", Some(1), Some(1))]
        );

        let banner = |path: &str| settings.app(path).map(|a| a.show_banner);
        settings.set_show_banner("Discord", Some(0)).unwrap();
        assert_eq!(banner("Discord"), Some(Some(0)));
        settings.set_show_banner("Discord", None).unwrap();
        assert_eq!(banner("Discord"), Some(None));
        // Removing twice is fine; missing keys are never created.
        settings.set_show_banner("Discord", None).unwrap();
        assert!(settings.set_show_banner("Missing", Some(0)).is_err());
        assert_eq!(banner("Missing"), None);
        assert_eq!(banner("Weird"), None);
        assert_eq!(banner(""), None);
        settings.set_show_banner(r"C:\Tools\app.exe", Some(0)).unwrap();
        assert_eq!(banner(r"C:\Tools\app.exe"), Some(Some(0)));
    }

    #[test]
    #[ignore = "diagnostic: prints this machine's notification settings"]
    fn prints_app_keys() {
        for app in Settings::default().apps() {
            println!("{:<70} enabled={:?} banner={:?}", app.path, app.enabled, app.show_banner);
        }
    }
}
