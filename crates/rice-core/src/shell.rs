//! Whitelist for `shell_open` and other user-supplied names.
//!
//! Allowed targets: `ms-settings:` URIs, `https://` URLs and absolute paths
//! inside the config directory (folders, or files with a harmless extension).
//! Everything else is `invalid_args`.

use crate::error::{BackendError, BackendResult};
use std::path::{Path, PathBuf};

/// A target that passed the syntactic part of the whitelist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellTarget {
    /// `ms-settings:` URI or `https://` URL, normalised.
    Uri(String),
    /// Absolute path; still needs [`path_allowed`] after canonicalisation.
    Path(PathBuf),
}

/// File types that open in a viewer or editor, never as a program.
const SAFE_EXTENSIONS: [&str; 5] = ["css", "json", "log", "toml", "txt"];

/// Classifies a `shell_open` target.
pub fn classify(target: &str) -> BackendResult<ShellTarget> {
    let target = target.trim();
    if target.is_empty() || target.len() > 2048 || target.chars().any(char::is_control) {
        return Err(BackendError::invalid_args("empty or malformed target"));
    }
    if let Some(rest) = strip_prefix_ignore_case(target, "ms-settings:") {
        let ok = rest.chars().all(|c| c.is_ascii_alphanumeric() || "-_.?=&%:".contains(c));
        return if ok {
            Ok(ShellTarget::Uri(format!("ms-settings:{rest}")))
        } else {
            Err(BackendError::invalid_args("malformed ms-settings URI"))
        };
    }
    if strip_prefix_ignore_case(target, "https://").is_some() {
        let url = url::Url::parse(target).map_err(|e| BackendError::invalid_args(format!("malformed URL: {e}")))?;
        return if url.scheme() == "https" && url.host().is_some() {
            Ok(ShellTarget::Uri(url.to_string()))
        } else {
            Err(BackendError::invalid_args("only https URLs are allowed"))
        };
    }
    let path = Path::new(target);
    if path.is_absolute() && !target.contains("://") {
        return Ok(ShellTarget::Path(path.to_path_buf()));
    }
    Err(BackendError::invalid_args("target is not on the whitelist"))
}

/// Whether a canonical path may be opened: inside `root` (canonical) and
/// either a folder or a file with a harmless extension.
pub fn path_allowed(path: &Path, root: &Path, is_dir: bool) -> bool {
    if !path.starts_with(root) {
        return false;
    }
    is_dir
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| SAFE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Turns a canonical Windows path back into the familiar form
/// (`\\?\C:\x` → `C:\x`, `\\?\UNC\srv\share` → `\\srv\share`) for the shell.
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{rest}")
    } else if let Some(rest) = text.strip_prefix("\\\\?\\") {
        rest.to_owned()
    } else {
        text.into_owned()
    }
}

/// Whether a settings page id is safe to put into `settings.html#<page>`.
pub fn is_valid_page(page: &str) -> bool {
    (1..=32).contains(&page.len()) && page.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn strip_prefix_ignore_case<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    let head = text.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix).then(|| &text[prefix.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;

    fn rejected(target: &str) -> bool {
        matches!(classify(target), Err(e) if e.code == ErrorCode::InvalidArgs)
    }

    #[test]
    fn accepts_settings_uris_and_https() {
        assert_eq!(classify("ms-settings:bluetooth").unwrap(), ShellTarget::Uri("ms-settings:bluetooth".into()));
        assert_eq!(classify("MS-Settings:network-wifi").unwrap(), ShellTarget::Uri("ms-settings:network-wifi".into()));
        assert_eq!(
            classify("https://github.com/johnnyjack123").unwrap(),
            ShellTarget::Uri("https://github.com/johnnyjack123".into())
        );
    }

    #[test]
    fn rejects_everything_else() {
        assert!(rejected(""));
        assert!(rejected("http://example.org"));
        assert!(rejected("file:///C:/Windows/System32/cmd.exe"));
        assert!(rejected("calc.exe"));
        assert!(rejected("ms-settings:bluetooth\" & calc"));
        assert!(rejected("https://"));
        assert!(rejected("javascript:alert(1)"));
        assert!(rejected("relative\\path"));
    }

    #[cfg(windows)]
    #[test]
    fn accepts_absolute_paths_for_a_later_check() {
        assert_eq!(classify("C:\\Users\\me\\AppData\\Roaming\\rice").unwrap(), ShellTarget::Path("C:\\Users\\me\\AppData\\Roaming\\rice".into()));
    }

    #[cfg(windows)]
    #[test]
    fn paths_must_stay_inside_the_root() {
        let root = Path::new("C:\\Users\\me\\AppData\\Roaming\\rice");
        assert!(path_allowed(root, root, true));
        assert!(path_allowed(&root.join("themes"), root, true));
        assert!(path_allowed(&root.join("config.json"), root, false));
        assert!(path_allowed(&root.join("themes\\x\\theme.CSS"), root, false));
        assert!(!path_allowed(&root.join("evil.exe"), root, false));
        assert!(!path_allowed(&root.join("evil.lnk"), root, false));
        assert!(!path_allowed(Path::new("C:\\Users\\me\\AppData\\Roaming\\rice-other"), root, true));
        assert!(!path_allowed(Path::new("C:\\Windows"), root, true));
    }

    #[test]
    fn strips_verbatim_prefixes() {
        assert_eq!(display_path(Path::new("\\\\?\\C:\\Users\\me\\rice")), "C:\\Users\\me\\rice");
        assert_eq!(display_path(Path::new("\\\\?\\UNC\\nas\\home\\rice")), "\\\\nas\\home\\rice");
        assert_eq!(display_path(Path::new("C:\\plain")), "C:\\plain");
    }

    #[test]
    fn validates_page_ids() {
        assert!(is_valid_page("calendar"));
        assert!(is_valid_page("theme-editor"));
        assert!(!is_valid_page(""));
        assert!(!is_valid_page("x';alert(1)//"));
        assert!(!is_valid_page("Calendar"));
    }
}
