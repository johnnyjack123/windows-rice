//! Rules for mirroring Windows notifications and for hiding Windows' own
//! banners while rice runs (docs/BACKEND.md §4 "Mitteilungen").

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Format version of the banner backup file.
pub const BACKUP_VERSION: u32 = 1;

/// Apps whose banners always stay: alarms and incoming calls must reach the
/// user even if the island is hidden or covered.
const EXEMPT_PREFIXES: [&str; 3] = ["microsoft.windowsalarms_", "windows.systemtoast.calling", "io.github.johnnyjack123.rice"];

/// The notification settings of one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppBanner {
    /// Key path below `…\Notifications\Settings`, usually the AUMID.
    pub path: String,
    /// `Enabled` (`Some(0)`: notifications of this app are off).
    pub enabled: Option<u32>,
    /// `ShowBanner` (`None`: not set, Windows shows banners).
    pub show_banner: Option<u32>,
}

/// The per-app banner switches: the registry on Windows, a map in tests.
pub trait BannerStore {
    /// Every app with notification settings.
    fn apps(&self) -> Vec<AppBanner>;
    /// One app; `None` if it has no settings (yet) or they are unreadable.
    fn app(&self, path: &str) -> Option<AppBanner>;
    /// Writes (`Some`) or removes (`None`) `ShowBanner` of an existing app.
    fn set_show_banner(&self, path: &str, value: Option<u32>) -> Result<(), String>;
}

/// Original `ShowBanner` values from before rice changed them, by key path
/// below `…\Notifications\Settings` (`None`: the value did not exist).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BannerBackup {
    pub version: u32,
    pub entries: BTreeMap<String, Option<u32>>,
}

impl BannerBackup {
    /// An empty backup in the current format.
    pub fn new() -> Self {
        Self { version: BACKUP_VERSION, entries: BTreeMap::new() }
    }

    /// Reads a backup file's content; unreadable content counts as empty.
    pub fn parse(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|_| Self::new())
    }
}

/// What restoring one entry does to the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restore {
    /// The value did not exist before: remove it again.
    Delete,
    /// Write the original value back.
    Set(u32),
}

/// How to restore an entry, or `None` to leave it: only values rice set to
/// 0 and that are still 0 go back; a change by the user in the meantime wins.
pub fn restore_action(original: Option<u32>, current: Option<u32>) -> Option<Restore> {
    if current != Some(0) {
        return None;
    }
    match original {
        None => Some(Restore::Delete),
        Some(0) => None,
        Some(value) => Some(Restore::Set(value)),
    }
}

/// Whether rice must write `ShowBanner = 0` for an app with these values.
/// Apps with notifications switched off entirely (`Enabled = 0`) are left alone.
pub fn needs_suppression(key: &str, enabled: Option<u32>, show_banner: Option<u32>) -> bool {
    enabled != Some(0) && show_banner != Some(0) && !is_exempt(key)
}

/// Alarms, calls and rice itself keep their banners.
pub fn is_exempt(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    EXEMPT_PREFIXES.iter().any(|prefix| key.starts_with(prefix))
}

/// Registry key path of an app below `…\Notifications\Settings`. Path AUMIDs
/// (`C:\…\app.exe`) are nested keys; forward slashes are kept as they are.
pub fn key_for(aumid: &str) -> Option<String> {
    let key = aumid.trim().trim_matches('\\');
    (!key.is_empty() && !key.split('\\').any(|part| part.is_empty() || part == "." || part == "..")).then(|| key.to_owned())
}

/// Title and body from the texts of a toast's generic binding: the first
/// non-empty text is the title, the others form the body (one per line).
pub fn split_texts(texts: &[String]) -> (String, String) {
    let mut lines = texts.iter().map(|t| t.trim()).filter(|t| !t.is_empty());
    let title = lines.next().unwrap_or_default().to_owned();
    let body = lines.collect::<Vec<_>>().join("\n");
    (title, body)
}

/// Ids that appeared and ids that vanished between two listings, each in
/// the order of the respective listing.
pub fn diff_ids<'a>(previous: &'a [String], current: &'a [String]) -> (Vec<&'a str>, Vec<&'a str>) {
    let before: HashSet<&str> = previous.iter().map(String::as_str).collect();
    let now: HashSet<&str> = current.iter().map(String::as_str).collect();
    let added = current.iter().map(String::as_str).filter(|id| !before.contains(id)).collect();
    let removed = previous.iter().map(String::as_str).filter(|id| !now.contains(id)).collect();
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_only_untouched_values() {
        assert_eq!(restore_action(None, Some(0)), Some(Restore::Delete));
        assert_eq!(restore_action(Some(1), Some(0)), Some(Restore::Set(1)));
        // The user switched banners back on (or removed the value) meanwhile.
        assert_eq!(restore_action(Some(1), Some(1)), None);
        assert_eq!(restore_action(None, None), None);
        // rice never changes a value that already was 0.
        assert_eq!(restore_action(Some(0), Some(0)), None);
    }

    #[test]
    fn suppresses_only_apps_that_show_banners() {
        assert!(needs_suppression("com.squirrel.Discord.Discord", None, None));
        assert!(needs_suppression("org.whispersystems.signal-desktop", Some(1), Some(1)));
        assert!(!needs_suppression("com.squirrel.Discord.Discord", None, Some(0)));
        assert!(!needs_suppression("Windows.SystemToast.StartupApp", Some(0), None));
        assert!(!needs_suppression("Microsoft.WindowsAlarms_8wekyb3d8bbwe!App", None, None));
        assert!(!needs_suppression("Windows.SystemToast.Calling", None, None));
        assert!(!needs_suppression("io.github.johnnyjack123.rice_1abc!rice", None, None));
        assert!(is_exempt("windows.systemtoast.calling.SystemAlertNotification"));
    }

    #[test]
    fn maps_aumids_to_keys() {
        assert_eq!(key_for("Microsoft.Todos_8wekyb3d8bbwe!App").as_deref(), Some("Microsoft.Todos_8wekyb3d8bbwe!App"));
        assert_eq!(key_for(r"C:\Tools\app.exe").as_deref(), Some(r"C:\Tools\app.exe"));
        assert_eq!(key_for(""), None);
        assert_eq!(key_for(r"a\\b"), None);
        assert_eq!(key_for(r"..\Explorer"), None);
    }

    #[test]
    fn reads_backups_leniently() {
        let mut backup = BannerBackup::new();
        backup.entries.insert("Discord".into(), None);
        backup.entries.insert("Signal".into(), Some(1));
        let json = serde_json::to_string(&backup).unwrap();
        assert_eq!(json, r#"{"version":1,"entries":{"Discord":null,"Signal":1}}"#);
        assert_eq!(BannerBackup::parse(&json), backup);
        assert_eq!(BannerBackup::parse("{ broken"), BannerBackup::new());
    }

    #[test]
    fn splits_toast_texts() {
        let texts = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(split_texts(&texts(&["Joni", "Hallo", "Wie gehts?"])), ("Joni".into(), "Hallo\nWie gehts?".into()));
        assert_eq!(split_texts(&texts(&["", " Titel ", ""])), ("Titel".into(), String::new()));
        assert_eq!(split_texts(&[]), (String::new(), String::new()));
    }

    #[test]
    fn diffs_listings() {
        let ids = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let (before, after) = (ids(&["1", "2", "3"]), ids(&["2", "3", "4", "5"]));
        assert_eq!(diff_ids(&before, &after), (vec!["4", "5"], vec!["1"]));
        assert_eq!(diff_ids(&after, &after), (vec![], vec![]));
    }
}
