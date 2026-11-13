//! Secrets in the Windows Credential Manager ("Generic credentials"),
//! encrypted per user by Windows. rice keeps no passwords in files.

use windows::Win32::Foundation::FILETIME;
use windows::Win32::Security::Credentials::{
    CRED_FLAGS, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW,
};
use windows::Win32::System::Com::CoCreateGuid;
use windows::core::{HSTRING, PWSTR};

/// Stores a secret under `target` (replacing an existing one).
pub fn write(target: &str, user: &str, secret: &str) -> windows::core::Result<()> {
    let mut target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    let mut user: Vec<u16> = user.encode_utf16().chain(Some(0)).collect();
    // UTF-16, like the passwords Windows itself stores, so the entry reads well in the Control Panel.
    let mut blob: Vec<u8> = secret.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let credential = CREDENTIALW {
        Flags: CRED_FLAGS(0),
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(target.as_mut_ptr()),
        Comment: PWSTR::null(),
        LastWritten: FILETIME::default(),
        CredentialBlobSize: u32::try_from(blob.len()).unwrap_or(u32::MAX),
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: std::ptr::null_mut(),
        TargetAlias: PWSTR::null(),
        UserName: PWSTR(user.as_mut_ptr()),
    };
    // SAFETY: every pointer refers to a local buffer that outlives the call.
    let result = unsafe { CredWriteW(&credential, 0) };
    blob.fill(0);
    result
}

/// Reads the secret stored under `target`.
pub fn read(target: &str) -> Option<String> {
    let mut credential: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: Windows allocates the credential; it is freed with CredFree below.
    unsafe {
        CredReadW(&HSTRING::from(target), CRED_TYPE_GENERIC, None, &mut credential).ok()?;
        let entry = &*credential;
        let bytes = if entry.CredentialBlob.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(entry.CredentialBlob, entry.CredentialBlobSize as usize)
        };
        let units: Vec<u16> = bytes.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        let secret = String::from_utf16(&units).ok();
        CredFree(credential.cast());
        secret
    }
}

/// Removes the secret stored under `target` (missing entries are fine).
pub fn delete(target: &str) {
    // SAFETY: NUL-terminated target name.
    let _ = unsafe { CredDeleteW(&HSTRING::from(target), CRED_TYPE_GENERIC, None) };
}

/// A random UUID in lower case (`CoCreateGuid`, version 4).
pub fn new_uuid() -> String {
    // SAFETY: no arguments.
    match unsafe { CoCreateGuid() } {
        Ok(guid) => format!("{guid:?}").to_ascii_lowercase(),
        Err(_) => format!("{:032x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_reads_and_deletes_secrets() {
        let target = format!("rice/test/{}", new_uuid());
        assert_eq!(read(&target), None);
        write(&target, "joni", "geheim-äöü-✓").unwrap();
        assert_eq!(read(&target).as_deref(), Some("geheim-äöü-✓"));
        write(&target, "joni", "neu").unwrap();
        assert_eq!(read(&target).as_deref(), Some("neu"));
        delete(&target);
        assert_eq!(read(&target), None);
    }

    #[test]
    fn makes_uuids() {
        let a = new_uuid();
        assert_eq!(a.len(), 36);
        assert_ne!(a, new_uuid());
        assert_eq!(a, a.to_ascii_lowercase());
    }
}
