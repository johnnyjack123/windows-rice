//! User themes: one folder per theme under `%APPDATA%\rice\themes`, each with
//! `theme.toml` (manifest) and `theme.css`.

use crate::fsutil::atomic_write;
use rice_core::contract::{UserTheme, UserThemeManifest};
use rice_core::theme::{check_user_theme_id, manifest_to_toml, parse_manifest};
use rice_core::{BackendError, BackendResult};
use std::fs;
use std::path::{Path, PathBuf};

const MANIFEST: &str = "theme.toml";
const STYLESHEET: &str = "theme.css";

/// Reads and writes the themes folder.
#[derive(Debug, Clone)]
pub struct ThemeStore {
    dir: PathBuf,
}

impl ThemeStore {
    /// Store for the given themes folder (created on first save).
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The themes folder.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every folder with a valid id, `theme.toml` and `theme.css`, sorted by id.
    /// Broken folders are skipped (and logged), never fatal.
    pub fn list(&self) -> Vec<UserTheme> {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return Vec::new();
        };
        let mut themes: Vec<UserTheme> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|entry| {
                let id = entry.file_name().to_str()?.to_owned();
                check_user_theme_id(&id).ok()?;
                match read_theme(&entry.path(), &id) {
                    Ok(theme) => Some(theme),
                    Err(reason) => {
                        log::warn!("themes: skipping {id}: {reason}");
                        None
                    }
                }
            })
            .collect();
        themes.sort_by(|a, b| a.id.cmp(&b.id));
        themes
    }

    /// Creates or overwrites a theme folder.
    pub fn save(&self, id: &str, manifest: &UserThemeManifest, css: &str) -> BackendResult<UserTheme> {
        check_user_theme_id(id)?;
        let folder = self.dir.join(id);
        let toml = manifest_to_toml(manifest).map_err(BackendError::internal)?;
        atomic_write(&folder.join(MANIFEST), toml.as_bytes())?;
        atomic_write(&folder.join(STYLESHEET), css.as_bytes())?;
        Ok(UserTheme { id: id.to_owned(), manifest: manifest.clone(), css: css.to_owned() })
    }

    /// Deletes a theme folder. Deleting a missing theme succeeds.
    pub fn delete(&self, id: &str) -> BackendResult<()> {
        check_user_theme_id(id)?;
        match fs::remove_dir_all(self.dir.join(id)) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }
}

fn read_theme(folder: &Path, id: &str) -> Result<UserTheme, String> {
    let read = |name: &str| {
        fs::read_to_string(folder.join(name))
            // Notepad's BOM would break the first CSS selector.
            .map(|text| text.trim_start_matches('\u{feff}').to_owned())
            .map_err(|e| format!("{name}: {e}"))
    };
    let manifest = parse_manifest(&read(MANIFEST)?).map_err(|e| format!("{MANIFEST}: {e}"))?;
    let css = read(STYLESHEET)?;
    Ok(UserTheme { id: id.to_owned(), manifest, css })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rice_core::ErrorCode;
    use rice_core::contract::Scheme;

    fn manifest(name: &str) -> UserThemeManifest {
        UserThemeManifest {
            name: name.into(),
            author: "Du".into(),
            version: "1.0.0".into(),
            schemes: vec![Scheme::Light],
            based_on: Some("paper".into()),
        }
    }

    #[test]
    fn saves_lists_and_deletes() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path().join("themes"));
        assert!(store.list().is_empty());
        store.save("zeta", &manifest("Zeta"), ".z {}").unwrap();
        let saved = store.save("mein-papier", &manifest("Mein Papier"), "[data-theme=\"mein-papier\"] {}").unwrap();
        assert_eq!(store.list().iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), ["mein-papier", "zeta"]);
        assert_eq!(store.list()[0], saved);
        store.delete("zeta").unwrap();
        store.delete("zeta").unwrap();
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn rejects_bad_and_builtin_ids() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path().to_path_buf());
        for id in ["paper", "../escape", "Upper", ""] {
            assert_eq!(store.save(id, &manifest("X"), "").unwrap_err().code, ErrorCode::InvalidArgs);
            assert_eq!(store.delete(id).unwrap_err().code, ErrorCode::InvalidArgs);
        }
    }

    #[test]
    fn skips_incomplete_folders() {
        let dir = tempfile::tempdir().unwrap();
        let store = ThemeStore::new(dir.path().to_path_buf());
        fs::create_dir_all(dir.path().join("no-css")).unwrap();
        fs::write(dir.path().join("no-css").join(MANIFEST), "name = \"A\"").unwrap();
        fs::create_dir_all(dir.path().join("bad-toml")).unwrap();
        fs::write(dir.path().join("bad-toml").join(MANIFEST), "name = ").unwrap();
        fs::write(dir.path().join("bad-toml").join(STYLESHEET), "").unwrap();
        fs::create_dir_all(dir.path().join("Not Valid")).unwrap();
        fs::write(dir.path().join("loose.css"), "").unwrap();
        assert!(store.list().is_empty());
    }
}
