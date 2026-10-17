//! `%APPDATA%\rice\cache.sqlite`: calendar objects, sync state and changes
//! made while offline.

use crate::events::bounds;
use crate::ical::Component;
use rice_core::contract::TaskPatch;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS objects (
    href TEXT PRIMARY KEY,
    calendar TEXT NOT NULL,
    etag TEXT,
    kind TEXT NOT NULL,
    ics TEXT NOT NULL,
    starts INTEGER,
    ends INTEGER
);
CREATE INDEX IF NOT EXISTS objects_calendar ON objects (calendar, kind);
CREATE TABLE IF NOT EXISTS sync (calendar TEXT PRIMARY KEY, token TEXT, ctag TEXT);
CREATE TABLE IF NOT EXISTS pending (id INTEGER PRIMARY KEY AUTOINCREMENT, account TEXT NOT NULL, op TEXT NOT NULL);
";

/// What a calendar object holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Event,
    Todo,
    Other,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Todo => "todo",
            Self::Other => "other",
        }
    }

    /// The kind of a parsed object.
    pub fn of(calendar: &Component) -> Self {
        if calendar.children_named("VEVENT").next().is_some() {
            Self::Event
        } else if calendar.children_named("VTODO").next().is_some() {
            Self::Todo
        } else {
            Self::Other
        }
    }
}

/// A cached calendar object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    /// Server path, also the object's id.
    pub href: String,
    pub calendar: String,
    pub etag: Option<String>,
    pub ics: String,
}

/// A change made while the server was unreachable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Pending {
    Create { href: String, ics: String },
    Update { href: String, patch: TaskPatch },
    Delete { href: String },
}

/// The cache database.
pub struct Cache {
    db: Mutex<Connection>,
}

impl Cache {
    /// Opens (or creates) the cache.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        Self::with(Connection::open(path)?)
    }

    /// A cache that lives in memory only (tests, or when the file is unusable).
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::with(Connection::open_in_memory()?)
    }

    fn with(db: Connection) -> rusqlite::Result<Self> {
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(SCHEMA)?;
        Ok(Self { db: Mutex::new(db) })
    }

    fn db(&self) -> MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Stores an object (parsed once to know its kind and time span).
    pub fn put(&self, calendar: &str, href: &str, etag: Option<&str>, ics: &str) -> rusqlite::Result<()> {
        let parsed = Component::parse(ics).ok();
        let kind = parsed.as_ref().map_or(Kind::Other, Kind::of);
        let (starts, ends) = parsed.as_ref().map_or((None, None), bounds);
        self.db().execute(
            "INSERT INTO objects (href, calendar, etag, kind, ics, starts, ends) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(href) DO UPDATE SET calendar = ?2, etag = ?3, kind = ?4, ics = ?5, starts = ?6, ends = ?7",
            params![href, calendar, etag, kind.as_str(), ics, starts, ends],
        )?;
        Ok(())
    }

    /// Removes an object.
    pub fn remove(&self, href: &str) -> rusqlite::Result<()> {
        self.db().execute("DELETE FROM objects WHERE href = ?1", params![href])?;
        Ok(())
    }

    /// Forgets a calendar completely (objects and sync state).
    pub fn remove_calendar(&self, calendar: &str) -> rusqlite::Result<()> {
        let db = self.db();
        db.execute("DELETE FROM objects WHERE calendar = ?1", params![calendar])?;
        db.execute("DELETE FROM sync WHERE calendar = ?1", params![calendar])?;
        Ok(())
    }

    /// One object.
    pub fn get(&self, href: &str) -> rusqlite::Result<Option<Object>> {
        self.db()
            .query_row("SELECT href, calendar, etag, ics FROM objects WHERE href = ?1", params![href], |r| {
                Ok(Object { href: r.get(0)?, calendar: r.get(1)?, etag: r.get(2)?, ics: r.get(3)? })
            })
            .optional()
    }

    /// href → ETag of every object of a calendar.
    pub fn etags(&self, calendar: &str) -> rusqlite::Result<HashMap<String, Option<String>>> {
        let db = self.db();
        let mut statement = db.prepare("SELECT href, etag FROM objects WHERE calendar = ?1")?;
        let rows = statement.query_map(params![calendar], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    }

    fn list(&self, sql: &str, calendars: &[String], extra: &[i64]) -> rusqlite::Result<Vec<Object>> {
        let mut out = Vec::new();
        let db = self.db();
        let mut statement = db.prepare(sql)?;
        for calendar in calendars {
            let mut values: Vec<&dyn rusqlite::ToSql> = vec![calendar];
            values.extend(extra.iter().map(|v| v as &dyn rusqlite::ToSql));
            let rows = statement.query_map(values.as_slice(), |r| {
                Ok(Object { href: r.get(0)?, calendar: r.get(1)?, etag: r.get(2)?, ics: r.get(3)? })
            })?;
            for row in rows {
                out.push(row?);
            }
        }
        Ok(out)
    }

    /// Event objects of these calendars that may touch `[from, to)` (Unix seconds).
    pub fn events(&self, calendars: &[String], from: i64, to: i64) -> rusqlite::Result<Vec<Object>> {
        self.list(
            "SELECT href, calendar, etag, ics FROM objects WHERE calendar = ?1 AND kind = 'event'
             AND (starts IS NULL OR starts < ?3) AND (ends IS NULL OR ends > ?2 OR ends = starts)",
            calendars,
            &[from, to],
        )
    }

    /// Task objects of these calendars.
    pub fn todos(&self, calendars: &[String]) -> rusqlite::Result<Vec<Object>> {
        self.list("SELECT href, calendar, etag, ics FROM objects WHERE calendar = ?1 AND kind = 'todo'", calendars, &[])
    }

    /// (sync token, CTag) stored for a calendar.
    pub fn sync_state(&self, calendar: &str) -> rusqlite::Result<(Option<String>, Option<String>)> {
        Ok(self
            .db()
            .query_row("SELECT token, ctag FROM sync WHERE calendar = ?1", params![calendar], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?
            .unwrap_or((None, None)))
    }

    /// Stores the sync state of a calendar.
    pub fn set_sync_state(&self, calendar: &str, token: Option<&str>, ctag: Option<&str>) -> rusqlite::Result<()> {
        self.db().execute(
            "INSERT INTO sync (calendar, token, ctag) VALUES (?1, ?2, ?3) ON CONFLICT(calendar) DO UPDATE SET token = ?2, ctag = ?3",
            params![calendar, token, ctag],
        )?;
        Ok(())
    }

    /// Queues a change for the next sync.
    pub fn queue(&self, account: &str, change: &Pending) -> rusqlite::Result<()> {
        let json = serde_json::to_string(change).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        self.db().execute("INSERT INTO pending (account, op) VALUES (?1, ?2)", params![account, json])?;
        Ok(())
    }

    /// Queued changes of an account, oldest first.
    pub fn pending(&self, account: &str) -> rusqlite::Result<Vec<(i64, Pending)>> {
        let db = self.db();
        let mut statement = db.prepare("SELECT id, op FROM pending WHERE account = ?1 ORDER BY id")?;
        let rows = statement.query_map(params![account], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        let mut out = Vec::new();
        for row in rows {
            let (id, json) = row?;
            match serde_json::from_str(&json) {
                Ok(change) => out.push((id, change)),
                Err(error) => log::warn!("calendar: dropping unreadable queued change {id}: {error}"),
            }
        }
        Ok(out)
    }

    /// Removes a replayed change.
    pub fn done(&self, id: i64) -> rusqlite::Result<()> {
        self.db().execute("DELETE FROM pending WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Drops every queued change of an account.
    pub fn clear_pending(&self, account: &str) -> rusqlite::Result<()> {
        self.db().execute("DELETE FROM pending WHERE account = ?1", params![account])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT: &str = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:e\r\nDTSTART:20261020T080000Z\r\nDTEND:20261020T090000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    const TODO: &str = "BEGIN:VCALENDAR\r\nBEGIN:VTODO\r\nUID:t\r\nSUMMARY:x\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    #[test]
    fn stores_and_filters_objects() {
        let cache = Cache::in_memory().unwrap();
        cache.put("c1", "/c1/e.ics", Some("\"1\""), EVENT).unwrap();
        cache.put("c1", "/c1/t.ics", None, TODO).unwrap();
        cache.put("c2", "/c2/e.ics", Some("\"2\""), EVENT).unwrap();
        let day = |h: i64| 1_792_454_400 + h * 3600; // 2026-10-20T00:00Z + h
        assert_eq!(cache.events(&["c1".into()], day(0), day(24)).unwrap().len(), 1);
        assert_eq!(cache.events(&["c1".into()], day(10), day(24)).unwrap().len(), 0);
        assert_eq!(cache.events(&["c1".into(), "c2".into()], day(0), day(24)).unwrap().len(), 2);
        assert_eq!(cache.todos(&["c1".into()]).unwrap()[0].href, "/c1/t.ics");
        assert_eq!(cache.etags("c1").unwrap().len(), 2);
        cache.put("c1", "/c1/e.ics", Some("\"3\""), EVENT).unwrap();
        assert_eq!(cache.get("/c1/e.ics").unwrap().unwrap().etag.as_deref(), Some("\"3\""));
        cache.remove_calendar("c1").unwrap();
        assert!(cache.get("/c1/t.ics").unwrap().is_none());
        assert!(cache.get("/c2/e.ics").unwrap().is_some());
    }

    #[test]
    fn keeps_sync_state_and_pending_changes() {
        let cache = Cache::in_memory().unwrap();
        assert_eq!(cache.sync_state("c").unwrap(), (None, None));
        cache.set_sync_state("c", Some("token-1"), None).unwrap();
        assert_eq!(cache.sync_state("c").unwrap().0.as_deref(), Some("token-1"));
        cache.queue("a", &Pending::Delete { href: "/x".into() }).unwrap();
        cache.queue("a", &Pending::Update { href: "/y".into(), patch: TaskPatch { due: Some(None), ..Default::default() } }).unwrap();
        let pending = cache.pending("a").unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[1].1, Pending::Update { href: "/y".into(), patch: TaskPatch { due: Some(None), ..Default::default() } });
        cache.done(pending[0].0).unwrap();
        assert_eq!(cache.pending("a").unwrap().len(), 1);
        assert!(cache.pending("other").unwrap().is_empty());
    }

    #[test]
    fn opens_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.sqlite");
        Cache::open(&path).unwrap().put("c", "/h", None, TODO).unwrap();
        assert!(Cache::open(&path).unwrap().get("/h").unwrap().is_some());
    }
}
