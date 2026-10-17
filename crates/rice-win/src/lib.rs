//! Thin, Tauri-independent wrappers around the Windows APIs the backend needs.
//!
//! Each function does one Windows thing and returns plain Rust data; policy
//! (what to emit when) lives in the app crate.

#![cfg(windows)]

pub mod appearance;
pub mod audio;
pub mod brightness;
pub mod com;
pub mod events;
pub mod fonts;
pub mod media;
pub mod monitors;
pub mod occlusion;
pub mod power;
pub mod shell;
pub mod wallpaper;
pub mod window;

/// Converts a NUL-terminated UTF-16 buffer to a `String`.
pub(crate) fn from_wide(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..len])
}

/// Formats a COLORREF (`0x00BBGGRR`) as "#rrggbb".
pub fn colorref_to_hex(value: u32) -> String {
    let r = value & 0xff;
    let g = (value >> 8) & 0xff;
    let b = (value >> 16) & 0xff;
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_colorrefs() {
        assert_eq!(colorref_to_hex(0x00d4_7800), "#0078d4");
        assert_eq!(colorref_to_hex(0), "#000000");
    }

    #[test]
    fn reads_wide_strings() {
        let wide: Vec<u16> = "DISPLAY1\0garbage".encode_utf16().collect();
        assert_eq!(from_wide(&wide), "DISPLAY1");
        assert_eq!(from_wide(&[]), "");
    }
}
