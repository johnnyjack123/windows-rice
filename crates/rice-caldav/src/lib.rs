//! CalDAV for rice (docs/BACKEND.md §4 "Kalender und Aufgaben"): Nextcloud
//! discovery, incremental sync into a SQLite cache, expanded event
//! occurrences and VTODO tasks.

pub mod accounts;
pub mod cache;
pub mod client;
pub mod discovery;
pub mod events;
pub mod ical;
pub mod sync;
pub mod todo;
pub mod webdav;
pub mod zones;
