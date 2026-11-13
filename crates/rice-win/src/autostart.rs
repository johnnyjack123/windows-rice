//! Starting with Windows through the per-user `Run` key. Task Manager's
//! "Startup apps" switch lives next to it in `StartupApproved\Run` (first
//! byte odd = disabled) and counts as well.

use crate::registry::{Key, message};
use windows::Win32::System::Registry::{KEY_QUERY_VALUE, KEY_SET_VALUE};

const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";

/// The autostart entry of one app.
#[derive(Debug, Clone)]
pub struct Autostart {
    name: String,
    run: String,
    approved: String,
}

impl Autostart {
    /// The entry called `name` (the value name in both keys).
    pub fn new(name: &str) -> Self {
        Self { name: name.to_owned(), run: RUN.to_owned(), approved: APPROVED.to_owned() }
    }

    /// The registered command line, whether or not Task Manager disabled it.
    pub fn command(&self) -> Option<String> {
        Key::open(&self.run, KEY_QUERY_VALUE)?.string(&self.name)
    }

    /// Whether Windows starts the app at sign-in.
    pub fn is_enabled(&self) -> bool {
        self.command().is_some() && !self.disabled_in_task_manager()
    }

    fn disabled_in_task_manager(&self) -> bool {
        Key::open(&self.approved, KEY_QUERY_VALUE)
            .and_then(|key| key.binary(&self.name))
            .and_then(|flags| flags.first().copied())
            .is_some_and(|flag| flag & 1 == 1)
    }

    /// Registers `command` and lifts a "disabled" mark from Task Manager.
    pub fn enable(&self, command: &str) -> Result<(), String> {
        self.set_command(command)?;
        self.clear_mark()
    }

    /// Replaces the registered command line, leaving Task Manager's switch alone.
    pub fn set_command(&self, command: &str) -> Result<(), String> {
        let run = Key::open(&self.run, KEY_SET_VALUE).ok_or("the Run key is not writable")?;
        run.set_string(&self.name, command).map_err(message)
    }

    /// Removes the registration (and Task Manager's mark).
    pub fn disable(&self) -> Result<(), String> {
        if let Some(run) = Key::open(&self.run, KEY_SET_VALUE) {
            run.delete(&self.name).map_err(message)?;
        }
        self.clear_mark()
    }

    fn clear_mark(&self) -> Result<(), String> {
        match Key::open(&self.approved, KEY_SET_VALUE) {
            Some(approved) => approved.delete(&self.name).map_err(message),
            // The key appears once Task Manager has stored a switch.
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::scratch::Scratch;
    use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_BINARY, RegCloseKey, RegOpenKeyExW, RegSetValueExW};
    use windows::core::HSTRING;

    fn mark(path: &str, name: &str, flag: u8) {
        let mut key = HKEY::default();
        unsafe {
            assert!(RegOpenKeyExW(HKEY_CURRENT_USER, &HSTRING::from(path), None, KEY_WRITE, &mut key).is_ok());
            let data = [flag, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            assert!(RegSetValueExW(key, &HSTRING::from(name), None, REG_BINARY, Some(&data)).is_ok());
            let _ = RegCloseKey(key);
        }
    }

    #[test]
    fn registers_and_follows_task_manager() {
        let scratch = Scratch::new();
        scratch.create("Run", &[]);
        scratch.create("Approved", &[]);
        let entry = Autostart { name: "rice".into(), run: format!(r"{}\Run", scratch.0), approved: format!(r"{}\Approved", scratch.0) };

        assert!(!entry.is_enabled());
        entry.enable(r#""C:\Apps\rice\rice.exe""#).unwrap();
        assert_eq!(entry.command().as_deref(), Some(r#""C:\Apps\rice\rice.exe""#));
        assert!(entry.is_enabled());

        // Switched off in Task Manager: still registered, but not enabled.
        mark(&entry.approved, "rice", 0x03);
        assert!(!entry.is_enabled());
        mark(&entry.approved, "rice", 0x02);
        assert!(entry.is_enabled());
        mark(&entry.approved, "rice", 0x03);
        // A new path keeps Task Manager's choice …
        entry.set_command(r#""D:\rice\rice.exe""#).unwrap();
        assert!(!entry.is_enabled());
        assert_eq!(entry.command().as_deref(), Some(r#""D:\rice\rice.exe""#));
        // … switching it on in rice lifts the mark.
        entry.enable(r#""C:\Apps\rice\rice.exe""#).unwrap();
        assert!(entry.is_enabled());

        entry.disable().unwrap();
        assert!(!entry.is_enabled());
        assert_eq!(entry.command(), None);
        entry.disable().unwrap();
    }
}
