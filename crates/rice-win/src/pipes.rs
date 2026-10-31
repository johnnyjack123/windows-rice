//! Named-pipe helpers for the coding-agent bridge (docs/BACKEND.md §4
//! "Coding-Agents"): a name per Windows session, a security descriptor that
//! admits only the current user, and the program behind a pipe.

use std::ffi::c_void;
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, OpenProcessToken};
use windows::core::{HSTRING, PWSTR};

/// Pipe of the running rice in this Windows session. Development builds use
/// their own, so `pnpm tauri dev` and the installed rice never answer each
/// other's hooks.
pub fn agent_pipe_name() -> String {
    let mut session = 0u32;
    // SAFETY: out-pointer to a local.
    let _ = unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) };
    let suffix = if cfg!(debug_assertions) { "-dev" } else { "" };
    format!(r"\\.\pipe\rice-agents-{session}{suffix}")
}

/// `SECURITY_ATTRIBUTES` that admit only the current user (and SYSTEM).
pub struct UserOnly {
    attributes: SECURITY_ATTRIBUTES,
    descriptor: PSECURITY_DESCRIPTOR,
}

// SAFETY: the descriptor is a heap block owned only by this value and never
// mutated after creation; moving it to another thread is fine.
unsafe impl Send for UserOnly {}

impl UserOnly {
    /// Builds the descriptor from the current user's SID.
    pub fn new() -> windows::core::Result<Self> {
        let sddl = HSTRING::from(format!("D:P(A;;GA;;;{})(A;;GA;;;SY)", current_user_sid()?));
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: NUL-terminated SDDL; the descriptor is freed in Drop.
        unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(&sddl, SDDL_REVISION_1, &mut descriptor, None)? };
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        Ok(Self { attributes, descriptor })
    }

    /// Pointer for `CreateNamedPipe`-style APIs; valid while `self` lives.
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        (&mut self.attributes as *mut SECURITY_ATTRIBUTES).cast()
    }
}

impl Drop for UserOnly {
    fn drop(&mut self) {
        // SAFETY: allocated by ConvertStringSecurityDescriptorToSecurityDescriptorW.
        let _ = unsafe { LocalFree(Some(HLOCAL(self.descriptor.0))) };
    }
}

/// SID of the user this process runs as, e.g. "S-1-5-21-…".
fn current_user_sid() -> windows::core::Result<String> {
    let mut token = HANDLE::default();
    // SAFETY: token of our own process, closed below; the buffer is u64-aligned
    // as TOKEN_USER requires and sized by the first call.
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let result = (|| {
            let mut len = 0u32;
            let _ = GetTokenInformation(token, TokenUser, None, 0, &mut len);
            let mut buffer = vec![0u64; (len as usize).div_ceil(8)];
            GetTokenInformation(token, TokenUser, Some(buffer.as_mut_ptr().cast()), len, &mut len)?;
            let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
            let mut text = PWSTR::null();
            ConvertSidToStringSidW(user.User.Sid, &mut text)?;
            let sid = text.to_string().map_err(|_| windows::core::Error::from_thread());
            let _ = LocalFree(Some(HLOCAL(text.0.cast())));
            sid
        })();
        let _ = CloseHandle(token);
        result
    }
}

/// Executable of the process serving the pipe behind `pipe` (a client end).
pub fn server_executable(pipe: &impl AsRawHandle) -> Option<PathBuf> {
    let mut pid = 0u32;
    // SAFETY: the caller keeps the pipe open for the call; out-pointer to a local.
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }.ok()?;
    crate::processes::exe_path(pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_user_only_descriptor() {
        let sid = current_user_sid().unwrap();
        assert!(sid.starts_with("S-1-5-"), "{sid}");
        let mut security = UserOnly::new().unwrap();
        assert!(!security.as_mut_ptr().is_null());
        let name = agent_pipe_name();
        assert!(name.starts_with(r"\\.\pipe\rice-agents-") && name.ends_with("-dev"), "{name}");
    }
}
