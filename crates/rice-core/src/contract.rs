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

/// One media session (GSMTC).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSession {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Cover as JPEG data URL; null if none.
    pub art_url: Option<String>,
    pub playing: bool,
    /// Position in seconds at `position_at`.
    pub position: f64,
    /// When the position was sampled (ISO 8601 with offset).
    pub position_at: String,
    pub duration: f64,
    pub can_previous: bool,
    pub can_next: bool,
    pub can_seek: bool,
}

/// All media sessions.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub sessions: Vec<MediaSession>,
    /// Session Windows considers current.
    pub current_id: Option<String>,
}

/// Transport action of `media_control`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaAction {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
}

/// Device categories with icons in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioDeviceKind {
    Speakers,
    Headphones,
    Hdmi,
    Bluetooth,
    Usb,
    Other,
}

/// Output or input device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub kind: AudioDeviceKind,
}

/// Audio endpoint state.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub volume: f64,
    pub muted: bool,
    pub device: Option<AudioDevice>,
    pub devices: Vec<AudioDevice>,
    pub mic_muted: bool,
    pub mic_available: bool,
}

/// Display state; null fields are unsupported on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayState {
    pub brightness: Option<f64>,
    pub night_light: Option<bool>,
}

/// Power state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerState {
    /// Null on machines without a battery.
    pub battery: Option<f64>,
    pub charging: bool,
    pub energy_saver: bool,
    pub energy_saver_supported: bool,
}

/// Session action of `power_action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PowerAction {
    Lock,
    Sleep,
    Signout,
    Restart,
    Shutdown,
}

/// CPU part of a [`SystemSample`].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuSample {
    /// Percent of total CPU, 0…100.
    pub usage: f64,
    pub name: String,
    pub ghz: Option<f64>,
}

/// Memory part of a [`SystemSample`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySample {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

/// GPU part of a [`SystemSample`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuSample {
    pub usage: f64,
    pub name: String,
    pub vram_used_bytes: u64,
    pub vram_total_bytes: u64,
}

/// Network part of a [`SystemSample`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSample {
    pub down_bytes_per_sec: f64,
    pub up_bytes_per_sec: f64,
}

/// One sample of system load (`system://sample`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SystemSample {
    pub time: String,
    pub cpu: CpuSample,
    pub memory: MemorySample,
    pub gpu: Option<GpuSample>,
    pub network: NetworkSample,
}

/// A running process (top entries only).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    /// Percent of total CPU, 0…100.
    pub cpu: f64,
    pub memory_bytes: u64,
    /// Icon as data URL; null → letter avatar.
    pub icon_url: Option<String>,
}

// ── Command arguments ────────────────────────────────────────────────────

/// `system_subscribe`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeArgs {
    pub interval_ms: f64,
}

/// `system_processes`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LimitArgs {
    pub limit: f64,
}

/// `system_kill`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PidArgs {
    pub pid: u32,
}

/// `media_control`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaControlArgs {
    pub session_id: Option<String>,
    pub action: MediaAction,
}

/// `media_seek`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSeekArgs {
    pub session_id: Option<String>,
    pub position: f64,
}

/// `audio_set_volume`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct VolumeArgs {
    pub volume: f64,
}

/// `audio_set_muted`, `audio_set_mic_muted`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct MutedArgs {
    pub muted: bool,
}

/// `audio_set_device`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceIdArgs {
    pub id: String,
}

/// `display_set_brightness`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BrightnessArgs {
    pub value: f64,
}

/// Every `{ enabled }` command (night light, energy saver …).
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct EnabledArgs {
    pub enabled: bool,
}

/// `power_action`.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PowerActionArgs {
    pub action: PowerAction,
}

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
    pub const MEDIA_STATE: &str = "media://state";
    pub const AUDIO_STATE: &str = "audio://state";
    pub const DISPLAY_STATE: &str = "display://state";
    pub const POWER_STATE: &str = "power://state";
    pub const SYSTEM_SAMPLE: &str = "system://sample";
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
    fn serialises_device_states() {
        let media = MediaState {
            current_id: Some("Spotify.exe".into()),
            sessions: vec![MediaSession {
                id: "Spotify.exe".into(),
                app_name: "Spotify".into(),
                title: "T".into(),
                artist: "A".into(),
                album: "B".into(),
                art_url: None,
                playing: true,
                position: 12.5,
                position_at: "2026-10-16T12:00:00.000+02:00".into(),
                duration: 200.0,
                can_previous: true,
                can_next: false,
                can_seek: true,
            }],
        };
        assert_eq!(
            serde_json::to_value(media).unwrap(),
            json!({
                "currentId": "Spotify.exe",
                "sessions": [{
                    "id": "Spotify.exe", "appName": "Spotify", "title": "T", "artist": "A", "album": "B", "artUrl": null,
                    "playing": true, "position": 12.5, "positionAt": "2026-10-16T12:00:00.000+02:00", "duration": 200.0,
                    "canPrevious": true, "canNext": false, "canSeek": true,
                }],
            })
        );
        let audio = AudioState {
            volume: 0.62,
            muted: false,
            device: Some(AudioDevice { id: "{0.0.0}".into(), name: "WH-1000XM5".into(), kind: AudioDeviceKind::Bluetooth }),
            devices: vec![],
            mic_muted: true,
            mic_available: true,
        };
        assert_eq!(
            serde_json::to_value(audio).unwrap(),
            json!({
                "volume": 0.62, "muted": false, "device": { "id": "{0.0.0}", "name": "WH-1000XM5", "kind": "bluetooth" },
                "devices": [], "micMuted": true, "micAvailable": true,
            })
        );
        assert_eq!(serde_json::to_value(DisplayState::default()).unwrap(), json!({ "brightness": null, "nightLight": null }));
        let power = PowerState { battery: None, charging: false, energy_saver: true, energy_saver_supported: false };
        assert_eq!(
            serde_json::to_value(power).unwrap(),
            json!({ "battery": null, "charging": false, "energySaver": true, "energySaverSupported": false })
        );
    }

    #[test]
    fn serialises_system_samples() {
        let sample = SystemSample {
            time: "2026-10-17T11:00:00.000+02:00".into(),
            cpu: CpuSample { usage: 12.5, name: "Snapdragon X Elite".into(), ghz: None },
            memory: MemorySample { used_bytes: 8, total_bytes: 16 },
            gpu: Some(GpuSample { usage: 3.0, name: "Adreno".into(), vram_used_bytes: 1, vram_total_bytes: 2 }),
            network: NetworkSample { down_bytes_per_sec: 1.5, up_bytes_per_sec: 0.0 },
        };
        assert_eq!(
            serde_json::to_value(sample).unwrap(),
            json!({
                "time": "2026-10-17T11:00:00.000+02:00",
                "cpu": { "usage": 12.5, "name": "Snapdragon X Elite", "ghz": null },
                "memory": { "usedBytes": 8, "totalBytes": 16 },
                "gpu": { "usage": 3.0, "name": "Adreno", "vramUsedBytes": 1, "vramTotalBytes": 2 },
                "network": { "downBytesPerSec": 1.5, "upBytesPerSec": 0.0 },
            })
        );
        let process = ProcessInfo { pid: 42, name: "Code.exe".into(), cpu: 3.1, memory_bytes: 9, icon_url: None };
        assert_eq!(
            serde_json::to_value(process).unwrap(),
            json!({ "pid": 42, "name": "Code.exe", "cpu": 3.1, "memoryBytes": 9, "iconUrl": null })
        );
        let subscribe: SubscribeArgs = serde_json::from_value(json!({ "intervalMs": 1000 })).unwrap();
        assert_eq!(subscribe.interval_ms, 1000.0);
    }

    #[test]
    fn parses_device_arguments() {
        let control: MediaControlArgs = serde_json::from_value(json!({ "sessionId": null, "action": "toggle" })).unwrap();
        assert_eq!((control.session_id, control.action), (None, MediaAction::Toggle));
        let seek: MediaSeekArgs = serde_json::from_value(json!({ "sessionId": "x", "position": 30.5 })).unwrap();
        assert_eq!(seek.session_id.as_deref(), Some("x"));
        let action: PowerActionArgs = serde_json::from_value(json!({ "action": "signout" })).unwrap();
        assert_eq!(action.action, PowerAction::Signout);
        assert!(serde_json::from_value::<PowerActionArgs>(json!({ "action": "format-c" })).is_err());
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
