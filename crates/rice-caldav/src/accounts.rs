//! `%APPDATA%\rice\accounts.json`: accounts and their calendars, without
//! passwords (those live in the Windows Credential Manager).

use crate::discovery::Remote;
use rice_core::contract::{AccountStatus, Calendar, CalendarAccount};
use rice_services::fsutil::atomic_write;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};
use url::Url;

/// Colour for calendars the server has none for (Nextcloud blue).
const DEFAULT_COLOR: &str = "#0082c9";

/// A calendar as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCalendar {
    pub id: String,
    pub name: String,
    pub color: String,
    pub url: String,
    pub supports_events: bool,
    pub supports_tasks: bool,
    pub enabled: bool,
}

/// An account as stored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccount {
    pub id: String,
    pub name: String,
    /// DAV root.
    pub url: String,
    pub user: String,
    /// Calendar home.
    pub home: String,
    pub status: AccountStatus,
    pub last_sync: Option<String>,
    pub calendars: Vec<StoredCalendar>,
}

impl StoredAccount {
    /// The contract view (no URLs of collections, no secrets).
    pub fn to_contract(&self) -> CalendarAccount {
        CalendarAccount {
            id: self.id.clone(),
            name: self.name.clone(),
            url: self.url.clone(),
            user: self.user.clone(),
            status: self.status,
            last_sync: self.last_sync.clone(),
            calendars: self
                .calendars
                .iter()
                .map(|c| Calendar {
                    id: c.id.clone(),
                    account_id: self.id.clone(),
                    name: c.name.clone(),
                    color: c.color.clone(),
                    supports_events: c.supports_events,
                    supports_tasks: c.supports_tasks,
                    enabled: c.enabled,
                })
                .collect(),
        }
    }
}

/// Stable id of an account (FNV-1a over user and address, so re-adding the
/// same account finds its cache again).
pub fn account_id(url: &str, user: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in format!("{user}@{url}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:012x}", hash & 0xffff_ffff_ffff)
}

/// Stable id of a calendar: account plus the collection's last path segment.
pub fn calendar_id(account: &str, url: &Url) -> String {
    let segment = url.path_segments().and_then(|mut s| s.rfind(|p| !p.is_empty()).map(str::to_owned)).unwrap_or_default();
    format!("{account}:{segment}")
}

/// Current calendars from the server, keeping ids and visibility of known ones.
pub fn merge_calendars(account: &str, existing: &[StoredCalendar], remote: &[Remote]) -> Vec<StoredCalendar> {
    remote
        .iter()
        .map(|r| {
            let id = calendar_id(account, &r.url);
            let enabled = existing.iter().find(|c| c.id == id).is_none_or(|c| c.enabled);
            StoredCalendar {
                id,
                name: r.name.clone(),
                color: r.color.clone().unwrap_or_else(|| DEFAULT_COLOR.to_owned()),
                url: r.url.to_string(),
                supports_events: r.events,
                supports_tasks: r.tasks,
                enabled,
            }
        })
        .collect()
}

/// The accounts file.
pub struct AccountStore {
    path: PathBuf,
    accounts: Mutex<Vec<StoredAccount>>,
}

impl AccountStore {
    /// Loads the file; an unreadable one starts empty (and is kept as a backup).
    pub fn open(path: PathBuf) -> Self {
        let accounts = match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Vec<StoredAccount>>(&text) {
                Ok(mut accounts) => {
                    // A sync that was running when the app ended is not running any more.
                    for account in &mut accounts {
                        if account.status == AccountStatus::Syncing {
                            account.status = AccountStatus::Ok;
                        }
                    }
                    accounts
                }
                Err(error) => {
                    log::warn!("calendar: {} unreadable ({error}); starting without accounts", path.display());
                    let _ = fs::copy(&path, path.with_extension("invalid.json"));
                    Vec::new()
                }
            },
            Err(_) => Vec::new(),
        };
        Self { path, accounts: Mutex::new(accounts) }
    }

    /// All accounts.
    pub fn list(&self) -> Vec<StoredAccount> {
        self.accounts.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// One account.
    pub fn get(&self, id: &str) -> Option<StoredAccount> {
        self.list().into_iter().find(|a| a.id == id)
    }

    /// Changes the accounts and saves them; returns the new list.
    pub fn update(&self, change: impl FnOnce(&mut Vec<StoredAccount>)) -> std::io::Result<Vec<StoredAccount>> {
        let mut accounts = self.accounts.lock().unwrap_or_else(PoisonError::into_inner);
        change(&mut accounts);
        let json = serde_json::to_string_pretty(&*accounts).map_err(std::io::Error::other)?;
        atomic_write(&self.path, json.as_bytes())?;
        Ok(accounts.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(path: &str, name: &str) -> Remote {
        Remote {
            url: Url::parse(&format!("https://cloud.example.org/remote.php/dav/calendars/joni/{path}/")).unwrap(),
            name: name.into(),
            color: None,
            events: true,
            tasks: false,
            ctag: None,
            sync_token: None,
        }
    }

    #[test]
    fn ids_are_stable() {
        let a = account_id("https://cloud.example.org/remote.php/dav/", "joni");
        assert_eq!(a, account_id("https://cloud.example.org/remote.php/dav/", "joni"));
        assert_ne!(a, account_id("https://cloud.example.org/remote.php/dav/", "anna"));
        assert_eq!(a.len(), 12);
        assert_eq!(calendar_id("a", &remote("personal", "x").url), "a:personal");
    }

    #[test]
    fn merging_keeps_visibility_and_adds_new_calendars() {
        let first = merge_calendars("a", &[], &[remote("personal", "Persönlich")]);
        assert!(first[0].enabled);
        assert_eq!(first[0].color, DEFAULT_COLOR);
        let mut hidden = first.clone();
        hidden[0].enabled = false;
        let merged = merge_calendars("a", &hidden, &[remote("personal", "Privat"), remote("work", "Arbeit")]);
        assert_eq!(merged.iter().map(|c| (c.id.as_str(), c.name.as_str(), c.enabled)).collect::<Vec<_>>(), [("a:personal", "Privat", false), ("a:work", "Arbeit", true)]);
    }

    #[test]
    fn persists_accounts_without_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("accounts.json");
        let store = AccountStore::open(path.clone());
        store
            .update(|accounts| {
                accounts.push(StoredAccount {
                    id: "a".into(),
                    name: "Nextcloud".into(),
                    url: "https://cloud.example.org/remote.php/dav/".into(),
                    user: "joni".into(),
                    home: "https://cloud.example.org/remote.php/dav/calendars/joni/".into(),
                    status: AccountStatus::Syncing,
                    last_sync: None,
                    calendars: merge_calendars("a", &[], &[remote("personal", "P")]),
                })
            })
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.to_lowercase().contains("password"));
        let reopened = AccountStore::open(path);
        assert_eq!(reopened.list()[0].status, AccountStatus::Ok, "a stale 'syncing' is reset");
        assert_eq!(reopened.get("a").unwrap().to_contract().calendars[0].account_id, "a");
    }
}
