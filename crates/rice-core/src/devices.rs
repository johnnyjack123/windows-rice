//! Pure mappings for audio, display and power values.

use crate::contract::{AudioDeviceKind, PowerState};

/// Rounds a 0…1 fraction to three decimals, so `0.62` set by the UI comes
/// back as `0.62` and not as the `f32` value `0.6200000047683716`.
pub fn round_fraction(value: f64) -> f64 {
    if value.is_finite() { (value.clamp(0.0, 1.0) * 1000.0).round() / 1000.0 } else { 0.0 }
}

/// Endpoint form factors (`EndpointFormFactor` in mmdeviceapi.h).
const FORM_SPEAKERS: u32 = 1;
const FORM_HEADPHONES: u32 = 3;
const FORM_HEADSET: u32 = 5;
const FORM_DISPLAY: u32 = 9;

/// Device category from the id of the driver the endpoint is connected to
/// (Bluetooth and USB show up there) and the endpoint's form factor.
pub fn audio_kind(connected_device_id: Option<&str>, form_factor: Option<u32>) -> AudioDeviceKind {
    let id = connected_device_id.unwrap_or_default().to_ascii_lowercase();
    if ["bthenum", "bthhfenum", "bthledevice", "bthle"].iter().any(|tag| id.contains(tag)) {
        return AudioDeviceKind::Bluetooth;
    }
    if id.contains("usb") {
        return AudioDeviceKind::Usb;
    }
    match form_factor {
        Some(FORM_HEADPHONES | FORM_HEADSET) => AudioDeviceKind::Headphones,
        Some(FORM_DISPLAY) => AudioDeviceKind::Hdmi,
        Some(FORM_SPEAKERS) => AudioDeviceKind::Speakers,
        _ => AudioDeviceKind::Other,
    }
}

/// DDC/CI brightness (`min…max`) as a fraction.
pub fn ddc_fraction(min: u32, current: u32, max: u32) -> Option<f64> {
    (max > min).then(|| round_fraction(f64::from(current.clamp(min, max) - min) / f64::from(max - min)))
}

/// Fraction as a DDC/CI value in `min…max`.
pub fn ddc_value(fraction: f64, min: u32, max: u32) -> u32 {
    let span = f64::from(max.saturating_sub(min));
    min + (round_fraction(fraction) * span).round() as u32
}

/// Fraction as a brightness percentage (WMI).
pub fn percent(fraction: f64) -> u8 {
    (round_fraction(fraction) * 100.0).round() as u8
}

/// `SYSTEM_POWER_STATUS` → contract state. `charging` means "on AC with a
/// battery": the UI shows the charging activity when a charger is plugged in.
/// Switching the energy saver has no documented API, so it is read-only.
pub fn power_state(ac_line_status: u8, battery_flag: u8, battery_percent: u8, system_status: u8) -> PowerState {
    const NO_BATTERY: u8 = 128;
    const UNKNOWN: u8 = 255;
    let battery = (battery_flag != NO_BATTERY && battery_flag != UNKNOWN && battery_percent <= 100)
        .then(|| f64::from(battery_percent) / 100.0);
    PowerState {
        battery,
        charging: battery.is_some() && ac_line_status == 1,
        energy_saver: system_status == 1,
        energy_saver_supported: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_fractions() {
        assert_eq!(round_fraction(f64::from(0.62f32)), 0.62);
        assert_eq!(round_fraction(1.4), 1.0);
        assert_eq!(round_fraction(-0.1), 0.0);
        assert_eq!(round_fraction(f64::NAN), 0.0);
    }

    #[test]
    fn classifies_audio_devices() {
        let bt = "{2}.\\\\?\\bthenum#{0000110b-0000-1000-8000-00805f9b34fb}_vid&0001004c#7&1d2e3f";
        assert_eq!(audio_kind(Some(bt), Some(FORM_HEADPHONES)), AudioDeviceKind::Bluetooth);
        assert_eq!(audio_kind(Some("{2}.\\\\?\\BTHHFENUM#{0000111e}"), None), AudioDeviceKind::Bluetooth);
        assert_eq!(audio_kind(Some("{2}.\\\\?\\usb#vid_046d&pid_0a87&mi_00"), Some(FORM_HEADSET)), AudioDeviceKind::Usb);
        let hd = Some("{1}.HDAUDIO\\FUNC_01&VEN_10EC&DEV_0257");
        assert_eq!(audio_kind(hd, Some(FORM_SPEAKERS)), AudioDeviceKind::Speakers);
        assert_eq!(audio_kind(hd, Some(FORM_HEADPHONES)), AudioDeviceKind::Headphones);
        assert_eq!(audio_kind(None, Some(FORM_DISPLAY)), AudioDeviceKind::Hdmi);
        assert_eq!(audio_kind(None, Some(2)), AudioDeviceKind::Other);
        assert_eq!(audio_kind(None, None), AudioDeviceKind::Other);
    }

    #[test]
    fn maps_ddc_ranges() {
        assert_eq!(ddc_fraction(0, 70, 100), Some(0.7));
        assert_eq!(ddc_fraction(10, 10, 10), None);
        assert_eq!(ddc_value(0.7, 0, 100), 70);
        assert_eq!(ddc_value(0.5, 20, 80), 50);
        assert_eq!(ddc_value(2.0, 0, 100), 100);
        assert_eq!(percent(0.663), 66);
    }

    #[test]
    fn maps_power_status() {
        let laptop = power_state(1, 8, 82, 0);
        assert_eq!(laptop, PowerState { battery: Some(0.82), charging: true, energy_saver: false, energy_saver_supported: false });
        let saver = power_state(0, 2, 15, 1);
        assert!(!saver.charging && saver.energy_saver);
        let desktop = power_state(1, 128, 255, 0);
        assert_eq!(desktop.battery, None);
        assert!(!desktop.charging);
        assert_eq!(power_state(255, 255, 255, 0).battery, None);
    }
}
