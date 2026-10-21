//! The per-app switch "Show notification banners" in the registry:
//! DWORD `ShowBanner` below `HKCU\…\Notifications\Settings\<AUMID>`
//! (docs/BACKEND.md §4 "Mitteilungen").
//!
//! Only `ShowBanner` is ever written, and only in keys that already exist;
//! policy and backups live in `rice-services`.

use crate::registry::{Key, message};
use rice_core::notifications::{AppBanner, BannerStore};
use windows::Win32::System::Registry::{KEY_QUERY_VALUE, KEY_READ, KEY_SET_VALUE};

/// Where Windows keeps the per-app notification settings (below HKCU).
pub const SETTINGS: &str = r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings";

/// Path AUMIDs nest a few levels deep; anything deeper is not an app.
const MAX_DEPTH: usize = 12;

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
        match value {
            Some(value) => key.set_dword("ShowBanner", value),
            None => key.delete("ShowBanner"),
        }
        .map_err(|error| format!("ShowBanner of {path}: {}", message(error)))
    }
}

fn read_app(key: &Key, path: &str) -> Option<AppBanner> {
    let show_banner = key.dword("ShowBanner").ok()?;
    let enabled = key.dword("Enabled").ok().flatten();
    Some(AppBanner { path: path.to_owned(), enabled, show_banner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::scratch::Scratch;

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
