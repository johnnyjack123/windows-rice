//! Installed font families (DirectWrite system collection).

use crate::from_wide;
use std::collections::BTreeSet;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection, IDWriteLocalizedStrings,
};
use windows::core::{BOOL, w};

/// Family names of all system fonts, sorted and unique (English names where available).
pub fn families() -> windows::core::Result<Vec<String>> {
    // SAFETY: DirectWrite factory creation and read-only collection queries.
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let mut collection: Option<IDWriteFontCollection> = None;
        factory.GetSystemFontCollection(&mut collection, false)?;
        let Some(collection) = collection else {
            return Ok(Vec::new());
        };
        let mut names = BTreeSet::new();
        for i in 0..collection.GetFontFamilyCount() {
            let Ok(family) = collection.GetFontFamily(i) else { continue };
            let Ok(strings) = family.GetFamilyNames() else { continue };
            if let Some(name) = english_name(&strings).filter(|n| !n.is_empty()) {
                names.insert(name);
            }
        }
        Ok(names.into_iter().collect())
    }
}

/// # Safety
/// `strings` must be a valid DirectWrite string list.
unsafe fn english_name(strings: &IDWriteLocalizedStrings) -> Option<String> {
    let mut index = 0u32;
    let mut exists = BOOL(0);
    // SAFETY: out-pointers to locals; index is checked against the count.
    unsafe {
        strings.FindLocaleName(w!("en-us"), &mut index, &mut exists).ok()?;
        if !exists.as_bool() {
            index = 0;
        }
        if index >= strings.GetCount() {
            return None;
        }
        let len = strings.GetStringLength(index).ok()?;
        let mut buffer = vec![0u16; len as usize + 1];
        strings.GetString(index, &mut buffer).ok()?;
        Some(from_wide(&buffer))
    }
}
