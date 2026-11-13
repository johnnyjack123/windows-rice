//! The few settings the backend itself needs, read from the stored JSON.
//!
//! Defaults and validation mirror `web/src/backend/settings.ts`; the frontend
//! stays the owner of the full schema.

use serde_json::Value;

/// Which monitor the island lives on (`settings.island.monitor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorSelection {
    Primary,
    Index(usize),
}

/// Backend view of the settings, always complete.
#[derive(Debug, Clone, PartialEq)]
pub struct BackendSettings {
    pub monitor: MonitorSelection,
    /// Distance of the island from the top of the work area, CSS px.
    pub offset_top: f64,
    /// Global shortcut that toggles the panel, e.g. "Super+Alt+Space".
    pub hotkey: String,
    /// "auto", "de" or "en".
    pub locale: String,
    pub hide_windows_banners: bool,
}

impl Default for BackendSettings {
    fn default() -> Self {
        Self {
            monitor: MonitorSelection::Primary,
            offset_top: 10.0,
            hotkey: "Super+Alt+Space".to_owned(),
            locale: "auto".to_owned(),
            hide_windows_banners: true,
        }
    }
}

impl BackendSettings {
    /// Reads the stored settings object; missing or mistyped values fall back to the defaults.
    pub fn from_stored(stored: &Value) -> Self {
        let defaults = Self::default();
        let island = &stored["island"];
        let monitor = match &island["monitor"] {
            Value::String(s) if s == "primary" => MonitorSelection::Primary,
            Value::Number(n) => n
                .as_u64()
                .and_then(|i| usize::try_from(i).ok())
                .map_or(MonitorSelection::Primary, MonitorSelection::Index),
            _ => defaults.monitor,
        };
        let offset_top = island["offsetTop"]
            .as_f64()
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(defaults.offset_top);
        let hotkey = island["hotkey"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map_or(defaults.hotkey, str::to_owned);
        let locale = match stored["locale"].as_str() {
            Some(l @ ("auto" | "de" | "en")) => l.to_owned(),
            _ => defaults.locale,
        };
        let hide_windows_banners = stored["notifications"]["hideWindowsBanners"]
            .as_bool()
            .unwrap_or(defaults.hide_windows_banners);
        Self { monitor, offset_top, hotkey, locale, hide_windows_banners }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_store_gives_defaults() {
        assert_eq!(BackendSettings::from_stored(&json!({})), BackendSettings::default());
        assert_eq!(BackendSettings::from_stored(&Value::Null), BackendSettings::default());
    }

    #[test]
    fn reads_valid_values() {
        let s = BackendSettings::from_stored(&json!({
            "locale": "en",
            "island": { "monitor": 1, "offsetTop": 24, "hotkey": "Ctrl+Alt+I" },
            "notifications": { "hideWindowsBanners": false },
        }));
        assert_eq!(s.monitor, MonitorSelection::Index(1));
        assert_eq!(s.offset_top, 24.0);
        assert_eq!(s.hotkey, "Ctrl+Alt+I");
        assert_eq!(s.locale, "en");
        assert!(!s.hide_windows_banners);
    }

    #[test]
    fn drops_mistyped_values() {
        let s = BackendSettings::from_stored(&json!({
            "locale": "fr",
            "island": { "monitor": -3, "offsetTop": "12", "hotkey": "  " },
            "notifications": { "hideWindowsBanners": "yes" },
        }));
        assert_eq!(s, BackendSettings::default());
        let fraction = BackendSettings::from_stored(&json!({ "island": { "monitor": 1.5 } }));
        assert_eq!(fraction.monitor, MonitorSelection::Primary);
    }
}
