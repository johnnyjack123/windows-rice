//! `%APPDATA%\rice\config.json`: an opaque JSON object the frontend owns.
//!
//! The backend never inserts defaults (they live in web/src/backend/settings.ts);
//! it only merges patches and persists atomically.

use crate::fsutil::atomic_write;
use rice_core::patch::apply_patch;
use serde_json::{Map, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Stored settings plus the file they live in.
#[derive(Debug)]
pub struct ConfigStore {
    path: PathBuf,
    value: Mutex<Value>,
}

impl ConfigStore {
    /// Loads the file. A missing file means `{}`; a broken one is copied to
    /// `config.invalid.json` first, so the next write does not lose it.
    pub fn open(path: PathBuf) -> Self {
        let value = match read_object(&path) {
            Ok(Some(value)) => value,
            Ok(None) => empty(),
            Err(reason) => {
                log::warn!("config: {} is not a JSON object ({reason}); starting empty", path.display());
                let backup = path.with_file_name("config.invalid.json");
                if let Err(error) = fs::copy(&path, &backup) {
                    log::warn!("config: could not back up the broken file: {error}");
                }
                empty()
            }
        };
        Self { path, value: Mutex::new(value) }
    }

    /// Path of `config.json`.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The stored object.
    pub fn get(&self) -> Value {
        self.lock().clone()
    }

    /// Merges a patch, writes the file and returns the new stored object.
    pub fn set(&self, patch: &Value) -> io::Result<Value> {
        let mut value = self.lock();
        let next = apply_patch(&value, patch);
        let text = serde_json::to_string_pretty(&next).map_err(io::Error::other)?;
        atomic_write(&self.path, text.as_bytes())?;
        *value = next.clone();
        Ok(next)
    }

    /// Re-reads the file after an external change. Returns the new object if
    /// it differs from what is in memory (own writes compare equal).
    pub fn reload(&self) -> Option<Value> {
        let disk = match read_object(&self.path) {
            Ok(Some(value)) => value,
            // Deleted: treat as reset to defaults.
            Ok(None) => empty(),
            Err(reason) => {
                // Editors save in steps; wait for a valid state.
                log::warn!("config: ignoring invalid external edit ({reason})");
                return None;
            }
        };
        let mut value = self.lock();
        if *value == disk {
            return None;
        }
        *value = disk.clone();
        Some(disk)
    }

    fn lock(&self) -> MutexGuard<'_, Value> {
        self.value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn empty() -> Value {
    Value::Object(Map::new())
}

/// `Ok(None)` when the file does not exist, `Err` when it is not a JSON object.
fn read_object(path: &Path) -> Result<Option<Value>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
        Ok(value @ Value::Object(_)) => Ok(Some(value)),
        Ok(_) => Err("top level is not an object".to_owned()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn starts_empty_and_persists_patches() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let store = ConfigStore::open(path.clone());
        assert_eq!(store.get(), json!({}));
        let stored = store.set(&json!({ "locale": "en", "island": { "autoHide": false } })).unwrap();
        assert_eq!(stored, json!({ "locale": "en", "island": { "autoHide": false } }));
        let stored = store.set(&json!({ "island": { "autoHide": null, "peek": false } })).unwrap();
        assert_eq!(stored, json!({ "locale": "en", "island": { "peek": false } }));
        let reopened = ConfigStore::open(path);
        assert_eq!(reopened.get(), stored);
    }

    #[test]
    fn reload_reports_only_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let store = ConfigStore::open(path.clone());
        store.set(&json!({ "locale": "de" })).unwrap();
        assert_eq!(store.reload(), None);
        fs::write(&path, "{ \"locale\": \"en\" }").unwrap();
        assert_eq!(store.reload(), Some(json!({ "locale": "en" })));
        fs::write(&path, "{ \"locale\": ").unwrap();
        assert_eq!(store.reload(), None);
        assert_eq!(store.get(), json!({ "locale": "en" }));
    }

    #[test]
    fn backs_up_a_broken_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, "[1, 2").unwrap();
        let store = ConfigStore::open(path.clone());
        assert_eq!(store.get(), json!({}));
        assert_eq!(fs::read_to_string(dir.path().join("config.invalid.json")).unwrap(), "[1, 2");
        // A BOM from Notepad is fine.
        fs::write(&path, "\u{feff}{\"locale\":\"de\"}").unwrap();
        assert_eq!(ConfigStore::open(path).get(), json!({ "locale": "de" }));
    }
}
