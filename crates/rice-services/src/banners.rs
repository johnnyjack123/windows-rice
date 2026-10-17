//! Hiding Windows' notification banners with a backup on disk, so they come
//! back on exit, after a crash (next start) and on uninstall
//! (docs/BACKEND.md §4 "Mitteilungen").
//!
//! The backup is always written before the registry; whatever the backup
//! does not cover is never changed.

use crate::fsutil::atomic_write;
use rice_core::notifications::{AppBanner, BannerBackup, BannerStore, Restore, needs_suppression, restore_action};
use std::io;
use std::path::Path;

/// File name of the backup in the config folder.
pub const BACKUP_FILE: &str = "banner-backup.json";

/// Hides the banners of those `apps` that show them. Returns how many apps
/// were switched; if the backup cannot be written, none are.
pub fn suppress(store: &impl BannerStore, backup_file: &Path, apps: Vec<AppBanner>) -> usize {
    let targets: Vec<AppBanner> = apps.into_iter().filter(|a| needs_suppression(&a.path, a.enabled, a.show_banner)).collect();
    if targets.is_empty() {
        return 0;
    }
    let mut backup = load(backup_file);
    for app in &targets {
        // An app already in the backup keeps its first original value.
        backup.entries.entry(app.path.clone()).or_insert(app.show_banner);
    }
    if let Err(error) = save(backup_file, &backup) {
        log::warn!("banners: backup not writable ({error}); Windows banners stay on");
        return 0;
    }
    targets
        .iter()
        .filter(|app| match store.set_show_banner(&app.path, Some(0)) {
            Ok(()) => true,
            Err(error) => {
                log::warn!("banners: {error}");
                false
            }
        })
        .count()
}

/// Brings back every banner rice hid that is still hidden, then deletes the
/// backup (entries that could not be restored stay in it). Returns how many
/// apps were restored.
pub fn restore(store: &impl BannerStore, backup_file: &Path) -> usize {
    let Ok(json) = std::fs::read_to_string(backup_file) else {
        return 0;
    };
    let mut left = BannerBackup::new();
    let mut restored = 0;
    for (path, original) in BannerBackup::parse(&json).entries {
        // Apps that are gone (uninstalled) need nothing.
        let Some(app) = store.app(&path) else {
            continue;
        };
        let result = match restore_action(original, app.show_banner) {
            Some(Restore::Delete) => store.set_show_banner(&path, None),
            Some(Restore::Set(value)) => store.set_show_banner(&path, Some(value)),
            None => continue,
        };
        match result {
            Ok(()) => restored += 1,
            Err(error) => {
                log::warn!("banners: {error}");
                left.entries.insert(path, original);
            }
        }
    }
    let cleaned = if left.entries.is_empty() {
        std::fs::remove_file(backup_file).or_else(|e| if e.kind() == io::ErrorKind::NotFound { Ok(()) } else { Err(e) })
    } else {
        save(backup_file, &left)
    };
    if let Err(error) = cleaned {
        log::warn!("banners: backup not updated ({error})");
    }
    restored
}

/// Whether a backup with entries exists (banners are or were hidden).
pub fn has_backup(backup_file: &Path) -> bool {
    !load(backup_file).entries.is_empty()
}

fn load(backup_file: &Path) -> BannerBackup {
    std::fs::read_to_string(backup_file).map(|json| BannerBackup::parse(&json)).unwrap_or_else(|_| BannerBackup::new())
}

fn save(backup_file: &Path, backup: &BannerBackup) -> io::Result<()> {
    let json = serde_json::to_vec_pretty(backup).map_err(io::Error::other)?;
    atomic_write(backup_file, &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    /// path → (Enabled, ShowBanner)
    type Values = BTreeMap<String, (Option<u32>, Option<u32>)>;

    /// Registry stand-in.
    #[derive(Default)]
    struct Fake {
        apps: RefCell<Values>,
        broken: Option<String>,
    }

    impl Fake {
        fn with(apps: &[(&str, Option<u32>, Option<u32>)]) -> Self {
            let map = apps.iter().map(|(p, e, s)| ((*p).to_owned(), (*e, *s))).collect();
            Self { apps: RefCell::new(map), broken: None }
        }

        fn banner(&self, path: &str) -> Option<u32> {
            self.apps.borrow()[path].1
        }

        fn set_by_user(&self, path: &str, value: Option<u32>) {
            self.apps.borrow_mut().get_mut(path).unwrap().1 = value;
        }
    }

    impl BannerStore for Fake {
        fn apps(&self) -> Vec<AppBanner> {
            self.apps
                .borrow()
                .iter()
                .map(|(path, (enabled, show_banner))| AppBanner { path: path.clone(), enabled: *enabled, show_banner: *show_banner })
                .collect()
        }

        fn app(&self, path: &str) -> Option<AppBanner> {
            self.apps().into_iter().find(|a| a.path == path)
        }

        fn set_show_banner(&self, path: &str, value: Option<u32>) -> Result<(), String> {
            if self.broken.as_deref() == Some(path) {
                return Err(format!("{path} is locked"));
            }
            self.apps.borrow_mut().get_mut(path).ok_or("missing")?.1 = value;
            Ok(())
        }
    }

    #[test]
    fn hides_and_restores_banners() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(BACKUP_FILE);
        let fake = Fake::with(&[
            ("Discord", None, None),
            ("Signal", Some(1), Some(1)),
            ("Quiet", None, Some(0)),
            ("Off", Some(0), None),
            ("Microsoft.WindowsAlarms_8wekyb3d8bbwe!App", None, None),
            ("Teams", None, None),
        ]);
        assert_eq!(suppress(&fake, &file, fake.apps()), 3);
        assert_eq!([fake.banner("Discord"), fake.banner("Signal"), fake.banner("Teams")], [Some(0); 3]);
        assert_eq!(fake.banner("Off"), None);
        assert_eq!(fake.banner("Microsoft.WindowsAlarms_8wekyb3d8bbwe!App"), None);
        assert!(has_backup(&file));
        // Again (e.g. a new app appeared): nothing to switch, originals stay.
        assert_eq!(suppress(&fake, &file, fake.apps()), 0);

        // The user switches Teams banners on again while rice runs.
        fake.set_by_user("Teams", Some(1));
        assert_eq!(restore(&fake, &file), 2);
        assert_eq!(fake.banner("Discord"), None);
        assert_eq!(fake.banner("Signal"), Some(1));
        assert_eq!(fake.banner("Quiet"), Some(0));
        assert_eq!(fake.banner("Teams"), Some(1));
        assert!(!file.exists());
        // Nothing left to do; a second restore is harmless.
        assert_eq!(restore(&fake, &file), 0);
    }

    #[test]
    fn keeps_failed_entries_for_the_next_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(BACKUP_FILE);
        let mut fake = Fake::with(&[("Discord", None, None), ("Signal", None, Some(1))]);
        assert_eq!(suppress(&fake, &file, fake.apps()), 2);
        fake.broken = Some("Signal".into());
        assert_eq!(restore(&fake, &file), 1);
        assert!(has_backup(&file));
        fake.broken = None;
        assert_eq!(restore(&fake, &file), 1);
        assert_eq!(fake.banner("Signal"), Some(1));
        assert!(!file.exists());
    }

    #[test]
    fn changes_nothing_without_a_backup() {
        let dir = tempfile::tempdir().unwrap();
        // A file where the backup's folder should be: the backup cannot be written.
        std::fs::write(dir.path().join("rice"), b"").unwrap();
        let file = dir.path().join("rice").join(BACKUP_FILE);
        let fake = Fake::with(&[("Discord", None, None)]);
        assert_eq!(suppress(&fake, &file, fake.apps()), 0);
        assert_eq!(fake.banner("Discord"), None);
    }

    #[test]
    fn skips_apps_that_disappeared() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(BACKUP_FILE);
        let fake = Fake::with(&[("Discord", None, None), ("Gone", None, None)]);
        suppress(&fake, &file, fake.apps());
        fake.apps.borrow_mut().remove("Gone");
        assert_eq!(restore(&fake, &file), 1);
        assert!(!file.exists());
    }
}
