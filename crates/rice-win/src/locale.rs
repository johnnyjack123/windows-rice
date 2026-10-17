//! The Windows display language, for the few backend strings with numbers
//! ("2,4 GHz"). The frontend resolves `locale = "auto"` the same way.

use windows::Win32::Globalization::GetUserDefaultUILanguage;

/// `LANG_GERMAN` primary language id.
const LANG_GERMAN: u16 = 0x07;

/// Whether Windows runs in German.
pub fn ui_is_german() -> bool {
    // SAFETY: no arguments.
    let language = unsafe { GetUserDefaultUILanguage() };
    language & 0x3ff == LANG_GERMAN
}
