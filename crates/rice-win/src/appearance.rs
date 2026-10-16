//! Windows appearance: dark mode, accent colour, animations, transparency.

use rice_core::contract::Appearance;
use windows::UI::ViewManagement::{UIColorType, UISettings};
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::WindowsAndMessaging::{SPI_GETCLIENTAREAANIMATION, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW};
use windows::core::{BOOL, w};

const FALLBACK_ACCENT: &str = "#0078d4";

/// Reads the current appearance. Missing values fall back to Windows defaults.
pub fn read() -> Appearance {
    let personalize = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    // SAFETY: constant, NUL-terminated key and value names.
    let light = unsafe { read_dword(personalize, w!("AppsUseLightTheme")) };
    let transparency = unsafe { read_dword(personalize, w!("EnableTransparency")) };
    Appearance {
        dark: light == Some(0),
        accent: accent().unwrap_or_else(|| FALLBACK_ACCENT.to_owned()),
        reduced_motion: !client_area_animation(),
        transparency: transparency != Some(0),
    }
}

fn accent() -> Option<String> {
    let settings = UISettings::new().ok()?;
    let c = settings.GetColorValue(UIColorType::Accent).ok()?;
    Some(format!("#{:02x}{:02x}{:02x}", c.R, c.G, c.B))
}

fn client_area_animation() -> bool {
    let mut enabled = BOOL(1);
    // SAFETY: SPI_GETCLIENTAREAANIMATION writes one BOOL.
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some(&mut enabled as *mut BOOL as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    ok.is_err() || enabled.as_bool()
}

/// Reads a DWORD below HKCU.
///
/// # Safety
/// Both strings must be NUL-terminated.
pub(crate) unsafe fn read_dword(key: windows::core::PCWSTR, value: windows::core::PCWSTR) -> Option<u32> {
    let mut data = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: out-buffer of exactly `size` bytes; the caller guarantees the strings.
    let status = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, key, value, RRF_RT_REG_DWORD, None, Some(&mut data as *mut u32 as *mut _), Some(&mut size))
    };
    (status == ERROR_SUCCESS).then_some(data)
}
