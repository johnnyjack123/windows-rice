//! Wi-Fi and Bluetooth mappings: signal bars, labels, profile XML, failure
//! reasons and device categories.

use crate::contract::{BluetoothKind, ConnectError, WifiNetwork};
use std::collections::HashMap;
use std::fmt::Write;

/// Signal quality (0…100) as 0…4 bars.
pub fn signal_bars(quality: u32) -> u8 {
    match quality {
        0 => 0,
        1..=25 => 1,
        26..=50 => 2,
        51..=75 => 3,
        _ => 4,
    }
}

/// Wi-Fi band of a channel centre frequency in kHz.
pub fn band_label(frequency_khz: u32, decimal_comma: bool) -> Option<&'static str> {
    match frequency_khz / 1000 {
        2400..=2500 => Some(if decimal_comma { "2,4 GHz" } else { "2.4 GHz" }),
        4900..=5899 => Some("5 GHz"),
        5925..=7125 => Some("6 GHz"),
        _ => None,
    }
}

/// Security as shown next to the band (`DOT11_AUTH_ALGORITHM`); `None` for open networks.
pub fn security_label(auth_algorithm: i32) -> Option<&'static str> {
    match auth_algorithm {
        2 => Some("WEP"),
        3 | 4 => Some("WPA"),
        6 => Some("WPA2-Enterprise"),
        7 => Some("WPA2"),
        8 | 11 => Some("WPA3-Enterprise"),
        9 => Some("WPA3"),
        10 => Some("OWE"),
        _ => None,
    }
}

/// "5 GHz · WPA3" from the known parts.
pub fn detail(band: Option<&str>, security: Option<&str>) -> String {
    [band, security].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}

/// One entry of the available-network list (Windows lists an SSID once per profile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedNetwork {
    pub ssid: String,
    pub quality: u32,
    pub secure: bool,
    pub known: bool,
    pub detail: String,
}

/// Strongest entry per SSID, without hidden networks and the connected one,
/// strongest first.
pub fn merge_scan(entries: Vec<ScannedNetwork>, connected_ssid: Option<&str>) -> Vec<WifiNetwork> {
    let mut by_ssid: HashMap<String, (ScannedNetwork, bool)> = HashMap::new();
    for entry in entries {
        if entry.ssid.is_empty() || Some(entry.ssid.as_str()) == connected_ssid {
            continue;
        }
        match by_ssid.get_mut(&entry.ssid) {
            Some((best, known)) => {
                *known |= entry.known;
                if entry.quality > best.quality {
                    *best = entry;
                }
            }
            None => {
                let known = entry.known;
                by_ssid.insert(entry.ssid.clone(), (entry, known));
            }
        }
    }
    let mut out: Vec<WifiNetwork> = by_ssid
        .into_values()
        .map(|(best, known)| WifiNetwork {
            ssid: best.ssid,
            signal: signal_bars(best.quality),
            secure: best.secure,
            known,
            detail: best.detail,
        })
        .collect();
    out.sort_by(|a, b| b.signal.cmp(&a.signal).then_with(|| a.ssid.to_lowercase().cmp(&b.ssid.to_lowercase())));
    out
}

/// Security of a profile the backend can create (personal networks only;
/// enterprise networks need credentials the panel does not ask for).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSecurity {
    Open,
    Wpa2Personal,
    Wpa3Personal,
}

/// Profile security for a network's default authentication algorithm.
pub fn profile_security(auth_algorithm: i32) -> Option<ProfileSecurity> {
    match auth_algorithm {
        1 => Some(ProfileSecurity::Open),
        // WPA2-Personal also joins WPA2/WPA3 transition networks.
        4 | 7 => Some(ProfileSecurity::Wpa2Personal),
        9 => Some(ProfileSecurity::Wpa3Personal),
        _ => None,
    }
}

/// Escapes text for XML element content.
pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// WLAN profile XML for a new network. The SSID goes in as hex (its raw
/// bytes) and as name; the key is stored by Windows, never by rice.
pub fn profile_xml(ssid: &[u8], name: &str, security: ProfileSecurity, key: Option<&str>) -> String {
    let hex = ssid.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02X}");
        s
    });
    let (authentication, encryption) = match security {
        ProfileSecurity::Open => ("open", "none"),
        ProfileSecurity::Wpa2Personal => ("WPA2PSK", "AES"),
        ProfileSecurity::Wpa3Personal => ("WPA3SAE", "AES"),
    };
    let shared_key = match (security, key) {
        (ProfileSecurity::Open, _) | (_, None) => String::new(),
        (_, Some(key)) => format!(
            "<sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{}</keyMaterial></sharedKey>",
            xml_escape(key)
        ),
    };
    let name = xml_escape(name);
    format!(
        "<?xml version=\"1.0\"?>\
<WLANProfile xmlns=\"http://www.microsoft.com/networking/WLAN/profile/v1\">\
<name>{name}</name>\
<SSIDConfig><SSID><hex>{hex}</hex><name>{name}</name></SSID></SSIDConfig>\
<connectionType>ESS</connectionType><connectionMode>auto</connectionMode>\
<MSM><security><authEncryption><authentication>{authentication}</authentication><encryption>{encryption}</encryption>\
<useOneX>false</useOneX></authEncryption>{shared_key}</security></MSM>\
</WLANProfile>"
    )
}

/// `WLAN_REASON_CODE_MSMSEC_BASE` … `_END`: security negotiation failed,
/// which for a personal network means the key is wrong.
const MSMSEC: std::ops::RangeInclusive<u32> = 0x0004_0000..=0x0004_FFFF;

/// Maps a failed connection's reason code.
pub fn connect_error(reason: u32) -> ConnectError {
    if MSMSEC.contains(&reason) { ConnectError::WrongKey } else { ConnectError::Failed }
}

/// Hints Windows reports for a Bluetooth device.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BluetoothHints {
    /// Classic Class of Device, major and minor parts.
    pub cod_major: Option<u32>,
    pub cod_minor: Option<u32>,
    /// BLE appearance category and subcategory.
    pub le_category: Option<u32>,
    pub le_subcategory: Option<u32>,
}

/// Device category from Class of Device / BLE appearance, refined by name.
pub fn bluetooth_kind(name: &str, hints: BluetoothHints) -> BluetoothKind {
    let kind = classic_kind(hints.cod_major, hints.cod_minor)
        .or_else(|| le_kind(hints.le_category, hints.le_subcategory))
        .unwrap_or(BluetoothKind::Other);
    let lower = name.to_lowercase();
    let in_ear = ["buds", "airpods", "earbud", "earphone"].iter().any(|word| lower.contains(word));
    match kind {
        BluetoothKind::Headphones | BluetoothKind::Other if in_ear => BluetoothKind::Earbuds,
        other => other,
    }
}

fn classic_kind(major: Option<u32>, minor: Option<u32>) -> Option<BluetoothKind> {
    const PHONE: u32 = 2;
    const AUDIO_VIDEO: u32 = 4;
    const PERIPHERAL: u32 = 5;
    let minor = minor.unwrap_or(0);
    match major? {
        PHONE => Some(BluetoothKind::Phone),
        // Headset, hands-free, headphones vs. loudspeaker, portable/car/hi-fi audio.
        AUDIO_VIDEO => match minor {
            1 | 2 | 6 => Some(BluetoothKind::Headphones),
            5 | 7 | 8 | 10 => Some(BluetoothKind::Speaker),
            _ => None,
        },
        // Bits 4–5: keyboard/pointing; bits 0–3: joystick, gamepad …
        PERIPHERAL => match ((minor >> 4) & 0b11, minor & 0b1111) {
            (1 | 3, _) => Some(BluetoothKind::Keyboard),
            (2, _) => Some(BluetoothKind::Mouse),
            (_, 1 | 2) => Some(BluetoothKind::Gamepad),
            _ => None,
        },
        _ => None,
    }
}

fn le_kind(category: Option<u32>, subcategory: Option<u32>) -> Option<BluetoothKind> {
    const PHONE: u32 = 0x001;
    const HID: u32 = 0x00F;
    const AUDIO_SINK: u32 = 0x021;
    const WEARABLE_AUDIO: u32 = 0x025;
    match (category?, subcategory.unwrap_or(0)) {
        (PHONE, _) => Some(BluetoothKind::Phone),
        (HID, 1) => Some(BluetoothKind::Keyboard),
        (HID, 2 | 9) => Some(BluetoothKind::Mouse),
        (HID, 3 | 4) => Some(BluetoothKind::Gamepad),
        (AUDIO_SINK, _) => Some(BluetoothKind::Speaker),
        (WEARABLE_AUDIO, 1) => Some(BluetoothKind::Earbuds),
        (WEARABLE_AUDIO, _) => Some(BluetoothKind::Headphones),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanned(ssid: &str, quality: u32, known: bool) -> ScannedNetwork {
        ScannedNetwork { ssid: ssid.into(), quality, secure: true, known, detail: String::new() }
    }

    #[test]
    fn maps_signal_quality_to_bars() {
        assert_eq!([0, 1, 25, 26, 50, 51, 75, 76, 100].map(signal_bars), [0, 1, 1, 2, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn labels_bands_and_security() {
        assert_eq!(band_label(2_437_000, true), Some("2,4 GHz"));
        assert_eq!(band_label(2_437_000, false), Some("2.4 GHz"));
        assert_eq!(band_label(5_180_000, true), Some("5 GHz"));
        assert_eq!(band_label(5_955_000, true), Some("6 GHz"));
        assert_eq!(band_label(0, true), None);
        assert_eq!(security_label(9), Some("WPA3"));
        assert_eq!(security_label(7), Some("WPA2"));
        assert_eq!(security_label(1), None);
        assert_eq!(detail(Some("5 GHz"), Some("WPA3")), "5 GHz · WPA3");
        assert_eq!(detail(Some("5 GHz"), None), "5 GHz");
        assert_eq!(detail(None, None), "");
    }

    #[test]
    fn merges_scan_results() {
        let merged = merge_scan(
            vec![
                scanned("Home", 90, true),
                scanned("Cafe", 40, false),
                scanned("Cafe", 70, false),
                scanned("Cafe", 10, true),
                scanned("", 99, false),
                scanned("Neighbour", 80, false),
            ],
            Some("Home"),
        );
        let names: Vec<_> = merged.iter().map(|n| (n.ssid.as_str(), n.signal, n.known)).collect();
        assert_eq!(names, [("Neighbour", 4, false), ("Cafe", 3, true)]);
    }

    #[test]
    fn writes_escaped_profiles() {
        let xml = profile_xml(b"Caf\xc3\xa9 & Co", "Café & Co", ProfileSecurity::Wpa2Personal, Some("p<a>ss&\"'"));
        assert!(xml.contains("<hex>436166C3A9202620436F</hex>"));
        assert!(xml.contains("<name>Café &amp; Co</name>"));
        assert!(xml.contains("<keyMaterial>p&lt;a&gt;ss&amp;&quot;&apos;</keyMaterial>"));
        assert!(xml.contains("<authentication>WPA2PSK</authentication><encryption>AES</encryption>"));
        let open = profile_xml(b"Free", "Free", ProfileSecurity::Open, Some("ignored"));
        assert!(open.contains("<authentication>open</authentication>"));
        assert!(!open.contains("sharedKey"));
        assert_eq!(profile_security(9), Some(ProfileSecurity::Wpa3Personal));
        assert_eq!(profile_security(6), None);
    }

    #[test]
    fn classifies_connection_failures() {
        assert_eq!(connect_error(0x48014), ConnectError::WrongKey);
        assert_eq!(connect_error(0x30001), ConnectError::Failed);
        assert_eq!(connect_error(1), ConnectError::Failed);
    }

    #[test]
    fn classifies_bluetooth_devices() {
        let classic = |major, minor| BluetoothHints { cod_major: Some(major), cod_minor: Some(minor), ..Default::default() };
        let le = |category, sub| BluetoothHints { le_category: Some(category), le_subcategory: Some(sub), ..Default::default() };
        assert_eq!(bluetooth_kind("WH-1000XM5", classic(4, 6)), BluetoothKind::Headphones);
        assert_eq!(bluetooth_kind("Galaxy Buds2 Pro", classic(4, 1)), BluetoothKind::Earbuds);
        assert_eq!(bluetooth_kind("JBL Flip 6", classic(4, 5)), BluetoothKind::Speaker);
        assert_eq!(bluetooth_kind("K380", classic(5, 0x10)), BluetoothKind::Keyboard);
        assert_eq!(bluetooth_kind("MX Master", classic(5, 0x20)), BluetoothKind::Mouse);
        assert_eq!(bluetooth_kind("Xbox Controller", classic(5, 2)), BluetoothKind::Gamepad);
        assert_eq!(bluetooth_kind("Pixel 8", classic(2, 3)), BluetoothKind::Phone);
        assert_eq!(bluetooth_kind("MX Keys", le(0x00F, 1)), BluetoothKind::Keyboard);
        assert_eq!(bluetooth_kind("Pebble", le(0x00F, 2)), BluetoothKind::Mouse);
        assert_eq!(bluetooth_kind("Buds", le(0x025, 1)), BluetoothKind::Earbuds);
        assert_eq!(bluetooth_kind("Thing", BluetoothHints::default()), BluetoothKind::Other);
        assert_eq!(bluetooth_kind("AirPods Pro", BluetoothHints::default()), BluetoothKind::Earbuds);
    }
}
