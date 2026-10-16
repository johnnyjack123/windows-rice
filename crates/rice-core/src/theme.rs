//! User theme ids and the `theme.toml` manifest format.

use crate::contract::{Scheme, UserThemeManifest};
use crate::error::{BackendError, BackendResult};
use serde::{Deserialize, Serialize};

/// Themes shipped with the app; user themes may not reuse their ids.
pub const BUILTIN_THEMES: [&str; 5] = ["obsidian", "monet", "glass", "terminal", "paper"];

/// Whether `id` matches `^[a-z0-9-]{1,48}$`.
pub fn is_valid_theme_id(id: &str) -> bool {
    (1..=48).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Accepts ids a user theme folder may have: valid and not built in.
pub fn check_user_theme_id(id: &str) -> BackendResult<()> {
    if !is_valid_theme_id(id) {
        return Err(BackendError::invalid_args(format!("invalid theme id {id:?}")));
    }
    if BUILTIN_THEMES.contains(&id) {
        return Err(BackendError::invalid_args(format!("{id} is a built-in theme")));
    }
    Ok(())
}

/// On-disk form of the manifest (snake_case keys, no nulls).
#[derive(Debug, Serialize, Deserialize)]
struct ManifestFile {
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    schemes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    based_on: Option<String>,
}

/// Parses `theme.toml`. Unknown schemes are skipped; only `name` is required.
pub fn parse_manifest(text: &str) -> Result<UserThemeManifest, String> {
    let file: ManifestFile = toml::from_str(text).map_err(|e| e.to_string())?;
    let schemes = file
        .schemes
        .iter()
        .filter_map(|s| match s.as_str() {
            "dark" => Some(Scheme::Dark),
            "light" => Some(Scheme::Light),
            _ => None,
        })
        .collect();
    Ok(UserThemeManifest {
        name: file.name,
        author: file.author,
        version: file.version,
        schemes,
        based_on: file.based_on.filter(|b| !b.is_empty()),
    })
}

/// Writes `theme.toml` text for a manifest.
pub fn manifest_to_toml(manifest: &UserThemeManifest) -> Result<String, String> {
    let file = ManifestFile {
        name: manifest.name.clone(),
        author: manifest.author.clone(),
        version: manifest.version.clone(),
        schemes: manifest
            .schemes
            .iter()
            .map(|s| match s {
                Scheme::Dark => "dark".to_owned(),
                Scheme::Light => "light".to_owned(),
            })
            .collect(),
        based_on: manifest.based_on.clone(),
    };
    toml::to_string(&file).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    #[test]
    fn validates_ids() {
        assert!(is_valid_theme_id("mein-papier"));
        assert!(is_valid_theme_id("a"));
        assert!(!is_valid_theme_id(""));
        assert!(!is_valid_theme_id("Mein Papier"));
        assert!(!is_valid_theme_id("..\\obsidian"));
        assert!(!is_valid_theme_id(&"x".repeat(49)));
        assert_eq!(check_user_theme_id("paper").unwrap_err().code, ErrorCode::InvalidArgs);
        assert!(check_user_theme_id("paper-2").is_ok());
    }

    #[test]
    fn parses_the_documented_manifest() {
        let text = "name = \"Mein Papier\"\nauthor = \"Du\"\nversion = \"1.0.0\"\nschemes = [\"light\"]\nbased_on = \"paper\"\n";
        let manifest = parse_manifest(text).unwrap();
        assert_eq!(
            manifest,
            UserThemeManifest {
                name: "Mein Papier".into(),
                author: "Du".into(),
                version: "1.0.0".into(),
                schemes: vec![Scheme::Light],
                based_on: Some("paper".into()),
            }
        );
    }

    #[test]
    fn is_lenient_with_optional_fields() {
        let manifest = parse_manifest("name = \"X\"\nschemes = [\"dark\", \"sepia\"]").unwrap();
        assert_eq!(manifest.schemes, vec![Scheme::Dark]);
        assert_eq!(manifest.based_on, None);
        assert!(parse_manifest("author = \"no name\"").is_err());
        assert!(parse_manifest("not toml").is_err());
    }

    #[test]
    fn round_trips() {
        let manifest = UserThemeManifest {
            name: "Nacht \"Blau\"".into(),
            author: "Du".into(),
            version: "1.2.0".into(),
            schemes: vec![Scheme::Dark, Scheme::Light],
            based_on: None,
        };
        let text = manifest_to_toml(&manifest).unwrap();
        assert!(!text.contains("based_on"));
        assert_eq!(parse_manifest(&text).unwrap(), manifest);
    }
}
