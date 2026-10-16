//! Wire types of `web/src/backend/contract.ts` (the single source of truth).
//!
//! Field names are camelCase on the wire; enum values match the contract's
//! string literals. Command inputs arrive as one `args` object per command.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Rectangle in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Static facts about the running app.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub themes_dir: String,
    pub config_dir: String,
}

/// Concrete colour scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Dark,
    Light,
}

/// Windows appearance settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub dark: bool,
    /// Windows accent colour, "#rrggbb".
    pub accent: String,
    pub reduced_motion: bool,
    pub transparency: bool,
}

/// How Windows fits the wallpaper to the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WallpaperFit {
    Fill,
    Fit,
    Stretch,
    Tile,
    Center,
    Span,
}

/// Width and height in CSS px.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// A point in CSS px.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Wallpaper of the monitor the island lives on.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WallpaperInfo {
    /// JPEG data URL; null for a solid colour.
    pub url: Option<String>,
    /// Solid background colour, "#rrggbb".
    pub color: String,
    pub fit: WallpaperFit,
    /// Monitor size in CSS px.
    pub monitor: Size,
    /// Top-left of the island window relative to the monitor, CSS px.
    pub window: Point,
}

/// A connected monitor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MonitorInfo {
    /// 0-based index as used by `island.monitor`.
    pub index: u32,
    pub name: String,
    pub primary: bool,
    /// Size in physical pixels.
    pub width: u32,
    pub height: u32,
}

/// Occlusion state of the island zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Occlusion {
    pub occluded: bool,
    pub fullscreen: bool,
}

/// Payload of `island://edge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EdgeState {
    pub active: bool,
}

/// Length class of a pomodoro break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BreakKind {
    Short,
    Long,
}

/// What the full-screen break window shows. The backend only stores and relays it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakOverlay {
    /// Epoch ms when the break ends; null while it is paused.
    pub ends_at: Option<f64>,
    pub remaining: f64,
    pub duration: f64,
    pub kind: BreakKind,
    pub round: f64,
    pub rounds: f64,
}

/// Manifest of a user theme (theme.toml in the theme folder).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserThemeManifest {
    pub name: String,
    pub author: String,
    pub version: String,
    pub schemes: Vec<Scheme>,
    pub based_on: Option<String>,
}

/// A user theme found in the themes folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserTheme {
    pub id: String,
    pub manifest: UserThemeManifest,
    /// Full CSS text of theme.css.
    pub css: String,
}

// ── Command arguments ────────────────────────────────────────────────────

/// `config_set`.
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigSetArgs {
    pub patch: Value,
}

/// `theme_save_user`.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeSaveArgs {
    pub id: String,
    pub manifest: UserThemeManifest,
    pub css: String,
}

/// `theme_delete_user`.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeIdArgs {
    pub id: String,
}

/// `island_configure`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct IslandConfigureArgs {
    pub width: f64,
    pub height: f64,
}

/// `island_set_hit_regions`.
#[derive(Debug, Clone, Deserialize)]
pub struct HitRegionsArgs {
    pub rects: Vec<Rect>,
}

/// `island_set_expanded`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct ExpandedArgs {
    pub expanded: bool,
}

/// `window_open_settings`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct OpenSettingsArgs {
    #[serde(default)]
    pub page: Option<String>,
}

/// `shell_open`.
#[derive(Debug, Clone, Deserialize)]
pub struct ShellOpenArgs {
    pub target: String,
}

/// Event names, exactly as in the contract's `Events` map.
pub mod events {
    pub const CONFIG_CHANGED: &str = "config://changed";
    pub const THEMES_CHANGED: &str = "theme://user-changed";
    pub const APPEARANCE: &str = "system://appearance";
    pub const WALLPAPER: &str = "system://wallpaper";
    pub const ISLAND_OCCLUSION: &str = "island://occlusion";
    pub const ISLAND_EDGE: &str = "island://edge";
    pub const ISLAND_FOCUS_LOST: &str = "island://focus-lost";
    pub const ISLAND_TOGGLE: &str = "island://toggle";
    pub const BREAK_STATE: &str = "break://state";
    pub const BREAK_FINISHED: &str = "break://finished";
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serialises_camel_case_and_nulls() {
        let info = AppInfo { version: "0.1.0".into(), themes_dir: "C:\\t".into(), config_dir: "C:\\c".into() };
        assert_eq!(serde_json::to_value(info).unwrap(), json!({ "version": "0.1.0", "themesDir": "C:\\t", "configDir": "C:\\c" }));

        let appearance = Appearance { dark: true, accent: "#0078d4".into(), reduced_motion: false, transparency: true };
        assert_eq!(
            serde_json::to_value(appearance).unwrap(),
            json!({ "dark": true, "accent": "#0078d4", "reducedMotion": false, "transparency": true })
        );

        let wallpaper = WallpaperInfo {
            url: None,
            color: "#1d1730".into(),
            fit: WallpaperFit::Center,
            monitor: Size { width: 1920.0, height: 1080.0 },
            window: Point { x: 512.0, y: 0.0 },
        };
        assert_eq!(
            serde_json::to_value(wallpaper).unwrap(),
            json!({ "url": null, "color": "#1d1730", "fit": "center", "monitor": { "width": 1920.0, "height": 1080.0 }, "window": { "x": 512.0, "y": 0.0 } })
        );

        let monitor = MonitorInfo { index: 1, name: "Dell".into(), primary: false, width: 1920, height: 1080 };
        assert_eq!(
            serde_json::to_value(monitor).unwrap(),
            json!({ "index": 1, "name": "Dell", "primary": false, "width": 1920, "height": 1080 })
        );
        assert_eq!(serde_json::to_value(Occlusion::default()).unwrap(), json!({ "occluded": false, "fullscreen": false }));
        assert_eq!(serde_json::to_value(EdgeState { active: true }).unwrap(), json!({ "active": true }));
    }

    #[test]
    fn round_trips_break_overlays_and_themes() {
        let overlay = json!({ "endsAt": null, "remaining": 1000, "duration": 300000, "kind": "long", "round": 4, "rounds": 4 });
        let parsed: BreakOverlay = serde_json::from_value(overlay).unwrap();
        assert_eq!(parsed.kind, BreakKind::Long);
        assert_eq!(parsed.ends_at, None);
        let back = serde_json::to_value(&parsed).unwrap();
        assert_eq!(back["endsAt"], Value::Null);
        assert_eq!(back["kind"], json!("long"));

        let theme = json!({
            "id": "mein-papier",
            "manifest": { "name": "Mein Papier", "author": "Du", "version": "1.0.0", "schemes": ["light"], "basedOn": "paper" },
            "css": "[data-theme=\"mein-papier\"] {}",
        });
        let parsed: UserTheme = serde_json::from_value(theme.clone()).unwrap();
        assert_eq!(parsed.manifest.schemes, vec![Scheme::Light]);
        assert_eq!(serde_json::to_value(parsed).unwrap(), theme);
    }

    #[test]
    fn parses_command_arguments() {
        let page: OpenSettingsArgs = serde_json::from_value(json!({})).unwrap();
        assert_eq!(page.page, None);
        let regions: HitRegionsArgs =
            serde_json::from_value(json!({ "rects": [{ "x": 1.5, "y": 10, "width": 200, "height": 38 }] })).unwrap();
        assert_eq!(regions.rects[0], Rect { x: 1.5, y: 10.0, width: 200.0, height: 38.0 });
        let save: ThemeSaveArgs = serde_json::from_value(json!({
            "id": "x", "css": "", "manifest": { "name": "X", "author": "", "version": "1", "schemes": [], "basedOn": null },
        }))
        .unwrap();
        assert_eq!(save.manifest.based_on, None);
    }
}
