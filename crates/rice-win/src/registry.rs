//! Small HKCU registry helper shared by the banner switches and autostart.

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, REG_DWORD, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE, RRF_RT_REG_BINARY, RRF_RT_REG_DWORD,
    RRF_RT_REG_SZ, RegCloseKey, RegDeleteValueW, RegEnumKeyExW, RegEnumValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};
use windows::core::{HSTRING, PCWSTR, PWSTR};

/// Longest registry key name plus NUL.
const NAME_CAPACITY: usize = 256;

/// An open key below HKCU, closed on drop.
pub(crate) struct Key(HKEY);

impl Key {
    /// Opens an existing key; `None` if it does not exist or access is denied.
    pub(crate) fn open(path: &str, access: REG_SAM_FLAGS) -> Option<Self> {
        let mut key = HKEY::default();
        // SAFETY: NUL-terminated path; the handle is closed in Drop.
        let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, access, &mut key) };
        (status == ERROR_SUCCESS).then_some(Self(key))
    }

    /// Names of the direct subkeys.
    pub(crate) fn subkeys(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut buffer = [0u16; NAME_CAPACITY];
        for index in 0.. {
            let mut len = buffer.len() as u32;
            // SAFETY: buffer of `len` characters; optional out-params unused.
            let status = unsafe { RegEnumKeyExW(self.0, index, Some(PWSTR(buffer.as_mut_ptr())), &mut len, None, None, None, None) };
            if status != ERROR_SUCCESS {
                break;
            }
            names.push(String::from_utf16_lossy(&buffer[..len as usize]));
        }
        names
    }

    /// Whether the key holds at least one value.
    pub(crate) fn has_values(&self) -> bool {
        // Value names may be up to 16383 characters.
        let mut buffer = vec![0u16; 16_384];
        let mut len = buffer.len() as u32;
        // SAFETY: buffer of `len` characters; type and data are not requested.
        let status = unsafe { RegEnumValueW(self.0, 0, Some(PWSTR(buffer.as_mut_ptr())), &mut len, None, None, None, None) };
        status == ERROR_SUCCESS || status == ERROR_MORE_DATA
    }

    /// `Ok(None)` when the value is missing, `Err` when it is not a DWORD.
    pub(crate) fn dword(&self, name: &str) -> Result<Option<u32>, WIN32_ERROR> {
        let mut data = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // SAFETY: open key; out-buffer of exactly `size` bytes.
        let status = unsafe {
            RegGetValueW(self.0, PCWSTR::null(), &HSTRING::from(name), RRF_RT_REG_DWORD, None, Some((&mut data as *mut u32).cast()), Some(&mut size))
        };
        match status {
            ERROR_SUCCESS => Ok(Some(data)),
            ERROR_FILE_NOT_FOUND => Ok(None),
            error => Err(error),
        }
    }

    /// A `REG_SZ` value; `None` if missing or of another type.
    pub(crate) fn string(&self, name: &str) -> Option<String> {
        let bytes = self.raw(name, RRF_RT_REG_SZ)?;
        let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        Some(crate::from_wide(&units))
    }

    /// A binary value; `None` if missing or of another type.
    pub(crate) fn binary(&self, name: &str) -> Option<Vec<u8>> {
        self.raw(name, RRF_RT_REG_BINARY)
    }

    fn raw(&self, name: &str, kind: windows::Win32::System::Registry::REG_ROUTINE_FLAGS) -> Option<Vec<u8>> {
        let name = HSTRING::from(name);
        let mut size = 0u32;
        // SAFETY: size query without a buffer.
        let status = unsafe { RegGetValueW(self.0, PCWSTR::null(), &name, kind, None, None, Some(&mut size)) };
        if status != ERROR_SUCCESS {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        // SAFETY: out-buffer of exactly `size` bytes.
        let status = unsafe { RegGetValueW(self.0, PCWSTR::null(), &name, kind, None, Some(data.as_mut_ptr().cast()), Some(&mut size)) };
        if status != ERROR_SUCCESS {
            return None;
        }
        data.truncate(size as usize);
        Some(data)
    }

    pub(crate) fn set_dword(&self, name: &str, value: u32) -> Result<(), WIN32_ERROR> {
        self.set(name, REG_DWORD, &value.to_le_bytes())
    }

    pub(crate) fn set_string(&self, name: &str, value: &str) -> Result<(), WIN32_ERROR> {
        let bytes: Vec<u8> = value.encode_utf16().chain(Some(0)).flat_map(u16::to_le_bytes).collect();
        self.set(name, REG_SZ, &bytes)
    }

    fn set(&self, name: &str, kind: REG_VALUE_TYPE, data: &[u8]) -> Result<(), WIN32_ERROR> {
        // SAFETY: open key; NUL-terminated value name; data of its own length.
        match unsafe { RegSetValueExW(self.0, &HSTRING::from(name), None, kind, Some(data)) } {
            ERROR_SUCCESS => Ok(()),
            error => Err(error),
        }
    }

    /// Removes a value; a missing value is fine.
    pub(crate) fn delete(&self, name: &str) -> Result<(), WIN32_ERROR> {
        // SAFETY: open key; NUL-terminated value name.
        match unsafe { RegDeleteValueW(self.0, &HSTRING::from(name)) } {
            ERROR_SUCCESS | ERROR_FILE_NOT_FOUND => Ok(()),
            error => Err(error),
        }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: the handle came from RegOpenKeyExW and is closed once.
        let _ = unsafe { RegCloseKey(self.0) };
    }
}

/// Human-readable text for a registry error.
pub(crate) fn message(error: WIN32_ERROR) -> String {
    windows::core::Error::from(error.to_hresult()).message()
}

/// Scratch keys below `HKCU\Software` for tests, removed on drop.
#[cfg(test)]
pub(crate) mod scratch {
    use super::*;
    use windows::Win32::System::Registry::{KEY_WRITE, REG_OPTION_NON_VOLATILE, RegCreateKeyExW, RegDeleteTreeW};

    pub(crate) struct Scratch(pub(crate) String);

    impl Scratch {
        pub(crate) fn new() -> Self {
            Self(format!(r"Software\rice-test-{}", crate::credentials::new_uuid()))
        }

        /// Creates `path` below the scratch key; `None` values are written as a string (a wrong type).
        pub(crate) fn create(&self, path: &str, values: &[(&str, Option<u32>)]) {
            let mut key = HKEY::default();
            unsafe {
                let full = HSTRING::from(if path.is_empty() { self.0.clone() } else { format!(r"{}\{path}", self.0) });
                let status = RegCreateKeyExW(HKEY_CURRENT_USER, &full, None, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None);
                assert_eq!(status, ERROR_SUCCESS);
                for (name, value) in values {
                    let name = HSTRING::from(*name);
                    let status = match value {
                        Some(v) => RegSetValueExW(key, &name, None, REG_DWORD, Some(&v.to_le_bytes())),
                        None => RegSetValueExW(key, &name, None, REG_SZ, Some(&[b'x', 0, 0, 0])),
                    };
                    assert_eq!(status, ERROR_SUCCESS);
                }
                let _ = RegCloseKey(key);
            }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            // Deletes the scratch key with everything below it.
            let _ = unsafe { RegDeleteTreeW(HKEY_CURRENT_USER, &HSTRING::from(&self.0)) };
        }
    }
}
