//! Calendar and tasks over CalDAV (docs/BACKEND.md §4 "Kalender und Aufgaben").
//! Accounts live in `accounts.json`, passwords in the Windows Credential
//! Manager, objects in `cache.sqlite`. Changes made offline are queued and
//! replayed by the next sync.

use crate::blocking::blocking;
use crate::state::AppState;
use rice_caldav::accounts::{AccountStore, StoredAccount, StoredCalendar, account_id, calendar_id, merge_calendars};
use rice_caldav::cache::{Cache, Pending};
use rice_caldav::client::{DavClient, DavError, Put};
use rice_caldav::discovery::{discover, list_calendars};
use rice_caldav::events::occurrences;
use rice_caldav::ical::Component;
use rice_caldav::sync::sync_calendar;
use rice_caldav::todo::{apply_patch, new_task, task};
use rice_core::contract::{AccountStatus, CalendarAccount, CalendarEvent, Task, TaskPatch, events};
use rice_core::time::{iso_local, now_iso};
use rice_core::{BackendError, BackendResult, ErrorCode};
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

const FIRST_SYNC: Duration = Duration::from_secs(3);
const SYNC_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Calendar service state.
pub struct Calendars {
    store: AccountStore,
    cache: Cache,
    /// One sync at a time.
    syncing: tokio::sync::Mutex<()>,
}

impl Calendars {
    /// Opens `accounts.json` and `cache.sqlite` in the config folder; an
    /// unusable cache file is replaced by one in memory (refilled by the next sync).
    pub fn open(dir: &Path) -> rusqlite::Result<Self> {
        let cache = match Cache::open(&dir.join("cache.sqlite")) {
            Ok(cache) => cache,
            Err(error) => {
                log::warn!("calendar: cache unusable ({error}); keeping it in memory");
                Cache::in_memory()?
            }
        };
        Ok(Self { store: AccountStore::open(dir.join("accounts.json")), cache, syncing: tokio::sync::Mutex::new(()) })
    }
}

fn credential(account: &str) -> String {
    format!("rice/caldav/{account}")
}

fn service(app: &AppHandle) -> &Calendars {
    &app.state::<AppState>().inner().calendar
}

/// Maps transport errors to the codes the settings page understands.
fn dav_error(error: DavError) -> BackendError {
    match error {
        DavError::Auth => BackendError::new(ErrorCode::AuthFailed, "wrong user name or app password"),
        DavError::Network(message) => BackendError::new(ErrorCode::Network, message),
        DavError::Timeout => BackendError::new(ErrorCode::Timeout, "the server did not answer"),
        DavError::NotFound => BackendError::new(ErrorCode::Network, "no CalDAV server found at this address"),
        DavError::Parse(message) => BackendError::invalid_args(message),
        other => BackendError::new(ErrorCode::Network, other.to_string()),
    }
}

fn status_of(error: &DavError) -> AccountStatus {
    match error {
        DavError::Auth => AccountStatus::AuthFailed,
        _ => AccountStatus::Offline,
    }
}

fn storage(error: impl std::fmt::Display) -> BackendError {
    BackendError::internal(format!("calendar storage: {error}"))
}

/// Sends `calendar://accounts`.
fn emit_accounts(app: &AppHandle) {
    let _ = app.emit(events::CALENDAR_ACCOUNTS, accounts(app));
}

fn set_status(app: &AppHandle, id: &str, status: AccountStatus, synced: bool) {
    let result = service(app).store.update(|accounts| {
        if let Some(account) = accounts.iter_mut().find(|a| a.id == id) {
            account.status = status;
            if synced {
                account.last_sync = Some(now_iso());
            }
        }
    });
    if let Err(error) = result {
        log::warn!("calendar: {error}");
    }
    emit_accounts(app);
}

/// Syncs at start and every five minutes.
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_SYNC).await;
        loop {
            sync_all(&handle).await;
            tokio::time::sleep(SYNC_INTERVAL).await;
        }
    });
}

/// `calendar_accounts`.
pub fn accounts(app: &AppHandle) -> Vec<CalendarAccount> {
    service(app).store.list().iter().map(StoredAccount::to_contract).collect()
}

/// `calendar_add_account`: discovers the calendars, stores the password in the
/// Credential Manager and starts the first sync.
pub async fn add_account(app: &AppHandle, name: String, url: String, user: String, password: String) -> BackendResult<CalendarAccount> {
    let user = user.trim().to_owned();
    if user.is_empty() || url.trim().is_empty() || password.is_empty() {
        return Err(BackendError::invalid_args("address, user name and app password are required"));
    }
    let client = DavClient::new(&user, &password).map_err(dav_error)?;
    let found = discover(&client, &url).await.map_err(dav_error)?;
    let id = account_id(found.dav.as_str(), &user);
    rice_win::credentials::write(&credential(&id), &user, &password).map_err(BackendError::internal)?;
    let calendars = service(app);
    let previous = calendars.store.get(&id);
    let account = StoredAccount {
        id: id.clone(),
        name: Some(name.trim()).filter(|n| !n.is_empty()).unwrap_or("Nextcloud").to_owned(),
        url: found.dav.to_string(),
        user,
        home: found.home.to_string(),
        status: AccountStatus::Syncing,
        last_sync: previous.as_ref().and_then(|p| p.last_sync.clone()),
        calendars: merge_calendars(&id, previous.as_ref().map_or(&[][..], |p| &p.calendars), &found.calendars),
    };
    calendars
        .store
        .update(|accounts| {
            accounts.retain(|a| a.id != account.id);
            accounts.push(account.clone());
        })
        .map_err(storage)?;
    emit_accounts(app);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move { sync_one(&handle, &id).await });
    Ok(account.to_contract())
}

/// `calendar_remove_account`: account, cache and password.
pub fn remove_account(app: &AppHandle, id: &str) -> BackendResult<()> {
    let calendars = service(app);
    let account = calendars.store.get(id).ok_or_else(|| BackendError::not_found(format!("no account {id}")))?;
    calendars.store.update(|accounts| accounts.retain(|a| a.id != id)).map_err(storage)?;
    for calendar in &account.calendars {
        calendars.cache.remove_calendar(&calendar.id).map_err(storage)?;
    }
    calendars.cache.clear_pending(id).map_err(storage)?;
    rice_win::credentials::delete(&credential(id));
    emit_accounts(app);
    let _ = app.emit(events::CALENDAR_CHANGED, ());
    Ok(())
}

/// `calendar_set_enabled`.
pub fn set_enabled(app: &AppHandle, calendar: &str, enabled: bool) -> BackendResult<()> {
    let mut found = false;
    service(app)
        .store
        .update(|accounts| {
            for c in accounts.iter_mut().flat_map(|a| a.calendars.iter_mut()).filter(|c| c.id == calendar) {
                c.enabled = enabled;
                found = true;
            }
        })
        .map_err(storage)?;
    if !found {
        return Err(BackendError::not_found(format!("no calendar {calendar}")));
    }
    emit_accounts(app);
    let _ = app.emit(events::CALENDAR_CHANGED, ());
    Ok(())
}

/// `calendar_sync` and the timer: every account, one after another.
pub async fn sync_all(app: &AppHandle) {
    let _running = service(app).syncing.lock().await;
    for account in service(app).store.list() {
        sync_locked(app, &account.id).await;
    }
}

async fn sync_one(app: &AppHandle, id: &str) {
    let _running = service(app).syncing.lock().await;
    sync_locked(app, id).await;
}

async fn sync_locked(app: &AppHandle, id: &str) {
    let calendars = service(app);
    let Some(account) = calendars.store.get(id) else { return };
    set_status(app, id, AccountStatus::Syncing, false);
    let Some(password) = rice_win::credentials::read(&credential(id)) else {
        log::warn!("calendar: no password stored for {}", account.user);
        set_status(app, id, AccountStatus::AuthFailed, false);
        return;
    };
    let (client, home) = match (DavClient::new(&account.user, &password), Url::parse(&account.home)) {
        (Ok(client), Ok(home)) => (client, home),
        _ => {
            set_status(app, id, AccountStatus::Offline, false);
            return;
        }
    };
    let mut changed = false;
    let outcome = async {
        changed |= replay(calendars, &client, &account).await?;
        let remote = list_calendars(&client, &home).await?;
        let merged = merge_calendars(id, &account.calendars, &remote);
        for gone in account.calendars.iter().filter(|old| !merged.iter().any(|c| c.id == old.id)) {
            calendars.cache.remove_calendar(&gone.id).map_err(|e| DavError::Parse(e.to_string()))?;
            changed = true;
        }
        if merged != account.calendars {
            let update = merged.clone();
            calendars
                .store
                .update(|accounts| {
                    if let Some(a) = accounts.iter_mut().find(|a| a.id == id) {
                        a.calendars = update;
                    }
                })
                .map_err(|e| DavError::Parse(e.to_string()))?;
        }
        for collection in &remote {
            let calendar = calendar_id(id, &collection.url);
            match sync_calendar(&client, &calendars.cache, &calendar, collection).await {
                Ok(c) => changed |= c,
                Err(error @ (DavError::Auth | DavError::Network(_) | DavError::Timeout)) => return Err(error),
                // One broken calendar must not stop the others.
                Err(error) => log::warn!("calendar: {calendar} not synced ({error})"),
            }
        }
        Ok::<(), DavError>(())
    }
    .await;
    match outcome {
        Ok(()) => set_status(app, id, AccountStatus::Ok, true),
        Err(error) => {
            log::info!("calendar: sync of {} stopped ({error})", account.user);
            set_status(app, id, status_of(&error), false);
        }
    }
    if changed {
        let _ = app.emit(events::CALENDAR_CHANGED, ());
    }
}

fn object_url(account: &StoredAccount, href: &str) -> Result<Url, DavError> {
    Url::parse(&account.url).and_then(|u| u.join(href)).map_err(|e| DavError::Parse(e.to_string()))
}

/// Replays changes made offline; stops at the first network error.
async fn replay(calendars: &Calendars, client: &DavClient, account: &StoredAccount) -> Result<bool, DavError> {
    let pending = calendars.cache.pending(&account.id).map_err(|e| DavError::Parse(e.to_string()))?;
    let replayed = !pending.is_empty();
    for (queued, change) in pending {
        let result = match &change {
            Pending::Create { href, ics } => match client.put(&object_url(account, href)?, ics, Put::Create).await {
                Ok(etag) => {
                    if let Ok(Some(object)) = calendars.cache.get(href) {
                        let _ = calendars.cache.put(&object.calendar, href, etag.as_deref(), &object.ics);
                    }
                    Ok(())
                }
                // Already there (a replay that was cut off before).
                Err(DavError::Precondition) => Ok(()),
                Err(error) => Err(error),
            },
            Pending::Update { href, patch } => update_remote(calendars, client, account, href, patch).await.map(|_| ()),
            Pending::Delete { href } => match client.delete(&object_url(account, href)?, None).await {
                Ok(()) | Err(DavError::NotFound) => Ok(()),
                Err(error) => Err(error),
            },
        };
        match result {
            Err(error @ (DavError::Auth | DavError::Network(_) | DavError::Timeout)) => return Err(error),
            Err(error) => log::warn!("calendar: dropping queued change {change:?} ({error})"),
            Ok(()) => {}
        }
        calendars.cache.done(queued).map_err(|e| DavError::Parse(e.to_string()))?;
    }
    Ok(replayed)
}

/// Writes a patch to the server, on top of the server's current version.
async fn update_remote(calendars: &Calendars, client: &DavClient, account: &StoredAccount, href: &str, patch: &TaskPatch) -> Result<String, DavError> {
    let url = object_url(account, href)?;
    let calendar = calendars.cache.get(href).ok().flatten().map(|o| o.calendar).unwrap_or_default();
    let (server_ics, etag) = client.get(&url).await?;
    let ics = apply_patch(&server_ics, patch, chrono::Utc::now()).map_err(DavError::Parse)?;
    let etag = etag.ok_or_else(|| DavError::Parse("the server sent no ETag".into()))?;
    let new_etag = client.put(&url, &ics, Put::Replace(&etag)).await?;
    calendars.cache.put(&calendar, href, new_etag.as_deref(), &ics).map_err(|e| DavError::Parse(e.to_string()))?;
    Ok(ics)
}

fn enabled_calendars(calendars: &Calendars, filter: impl Fn(&StoredCalendar) -> bool) -> Vec<String> {
    calendars.store.list().iter().flat_map(|a| a.calendars.iter()).filter(|c| c.enabled && filter(c)).map(|c| c.id.clone()).collect()
}

fn account_of(calendars: &Calendars, calendar: &str) -> Option<(StoredAccount, StoredCalendar)> {
    calendars.store.list().into_iter().find_map(|a| {
        let found = a.calendars.iter().find(|c| c.id == calendar).cloned()?;
        Some((a, found))
    })
}

fn parse_instant(value: &str) -> BackendResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|t| t.with_timezone(&chrono::Utc))
        .map_err(|_| BackendError::invalid_args(format!("not an ISO date-time: {value}")))
}

/// `calendar_events`: occurrences of enabled calendars, sorted by start.
pub async fn events(app: &AppHandle, from: &str, to: &str) -> BackendResult<Vec<CalendarEvent>> {
    let (from, to) = (parse_instant(from)?, parse_instant(to)?);
    let handle = app.clone();
    blocking(move || {
        let calendars = service(&handle);
        let ids = enabled_calendars(calendars, |c| c.supports_events);
        let objects = calendars.cache.events(&ids, from.timestamp(), to.timestamp()).map_err(storage)?;
        let mut out: Vec<CalendarEvent> = objects
            .iter()
            .filter_map(|object| Component::parse(&object.ics).ok().map(|parsed| (object, parsed)))
            .flat_map(|(object, parsed)| {
                occurrences(&parsed, from, to).into_iter().map(move |o| CalendarEvent {
                    id: o.id(),
                    calendar_id: object.calendar.clone(),
                    title: o.title,
                    start: iso_local(o.start.timestamp_millis()),
                    end: iso_local(o.end.timestamp_millis()),
                    all_day: o.all_day,
                    location: o.location,
                })
            })
            .collect();
        out.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(out)
    })
    .await
}

/// `tasks_list`: open tasks first, then by due date and title.
pub async fn tasks(app: &AppHandle) -> BackendResult<Vec<Task>> {
    let handle = app.clone();
    blocking(move || {
        let calendars = service(&handle);
        let ids = enabled_calendars(calendars, |c| c.supports_tasks);
        let mut out: Vec<Task> = calendars
            .cache
            .todos(&ids)
            .map_err(storage)?
            .iter()
            .filter_map(|o| Component::parse(&o.ics).ok().and_then(|c| task(&c, &o.href, &o.calendar)))
            .collect();
        out.sort_by(|a, b| {
            a.done
                .cmp(&b.done)
                .then_with(|| a.due.is_none().cmp(&b.due.is_none()))
                .then_with(|| a.due.cmp(&b.due))
                .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });
        Ok(out)
    })
    .await
}

fn client_for(account: &StoredAccount) -> BackendResult<DavClient> {
    let password = rice_win::credentials::read(&credential(&account.id))
        .ok_or_else(|| BackendError::new(ErrorCode::AuthFailed, "no password stored; add the account again"))?;
    DavClient::new(&account.user, &password).map_err(dav_error)
}

/// A write that failed because the server is unreachable is queued; others fail.
fn queue_or_fail(app: &AppHandle, account: &StoredAccount, error: DavError, change: Pending) -> BackendResult<()> {
    if error.is_offline() {
        service(app).cache.queue(&account.id, &change).map_err(storage)?;
        set_status(app, &account.id, AccountStatus::Offline, false);
        return Ok(());
    }
    if error == DavError::Auth {
        set_status(app, &account.id, AccountStatus::AuthFailed, false);
    }
    Err(dav_error(error))
}

fn task_of(ics: &str, href: &str, calendar: &str) -> BackendResult<Task> {
    Component::parse(ics).ok().and_then(|c| task(&c, href, calendar)).ok_or_else(|| BackendError::internal("task unreadable"))
}

/// `tasks_create`.
pub async fn create_task(app: &AppHandle, calendar: &str, title: &str, due: Option<&str>) -> BackendResult<Task> {
    let title = title.trim();
    if title.is_empty() {
        return Err(BackendError::invalid_args("a task needs a title"));
    }
    let calendars = service(app);
    let (account, list) = account_of(calendars, calendar).ok_or_else(|| BackendError::not_found(format!("no task list {calendar}")))?;
    if !list.supports_tasks {
        return Err(BackendError::invalid_args(format!("{} holds no tasks", list.name)));
    }
    let uid = rice_win::credentials::new_uuid();
    let url = Url::parse(&list.url).and_then(|u| u.join(&format!("{uid}.ics"))).map_err(BackendError::internal)?;
    let href = url.path().to_owned();
    let ics = new_task(&uid, title, due, chrono::Utc::now()).map_err(BackendError::invalid_args)?;
    let client = client_for(&account)?;
    match client.put(&url, &ics, Put::Create).await {
        Ok(etag) => calendars.cache.put(calendar, &href, etag.as_deref(), &ics).map_err(storage)?,
        Err(error) => {
            queue_or_fail(app, &account, error, Pending::Create { href: href.clone(), ics: ics.clone() })?;
            calendars.cache.put(calendar, &href, None, &ics).map_err(storage)?;
        }
    }
    let _ = app.emit(events::TASKS_CHANGED, ());
    task_of(&ics, &href, calendar)
}

/// `tasks_update`: on the server's current version, so concurrent edits from
/// the phone are not overwritten.
pub async fn update_task(app: &AppHandle, href: &str, patch: TaskPatch) -> BackendResult<Task> {
    let calendars = service(app);
    let object = calendars.cache.get(href).map_err(storage)?.ok_or_else(|| BackendError::not_found(format!("no task {href}")))?;
    let (account, _) = account_of(calendars, &object.calendar).ok_or_else(|| BackendError::not_found("the task's list is gone"))?;
    let local = apply_patch(&object.ics, &patch, chrono::Utc::now()).map_err(BackendError::invalid_args)?;
    let client = client_for(&account)?;
    let url = object_url(&account, href).map_err(dav_error)?;
    let attempt = match object.etag.as_deref() {
        Some(etag) => match client.put(&url, &local, Put::Replace(etag)).await {
            Ok(new_etag) => {
                calendars.cache.put(&object.calendar, href, new_etag.as_deref(), &local).map_err(storage)?;
                Ok(local.clone())
            }
            // Changed elsewhere: apply the patch to the server's version.
            Err(DavError::Precondition) => update_remote(calendars, &client, &account, href, &patch).await,
            Err(error) => Err(error),
        },
        None => update_remote(calendars, &client, &account, href, &patch).await,
    };
    let ics = match attempt {
        Ok(ics) => ics,
        Err(error) => {
            queue_or_fail(app, &account, error, Pending::Update { href: href.to_owned(), patch })?;
            calendars.cache.put(&object.calendar, href, object.etag.as_deref(), &local).map_err(storage)?;
            local
        }
    };
    let _ = app.emit(events::TASKS_CHANGED, ());
    task_of(&ics, href, &object.calendar)
}

/// `tasks_delete`.
pub async fn delete_task(app: &AppHandle, href: &str) -> BackendResult<()> {
    let calendars = service(app);
    let Some(object) = calendars.cache.get(href).map_err(storage)? else {
        return Ok(());
    };
    let (account, _) = account_of(calendars, &object.calendar).ok_or_else(|| BackendError::not_found("the task's list is gone"))?;
    let client = client_for(&account)?;
    let url = object_url(&account, href).map_err(dav_error)?;
    let result = match client.delete(&url, object.etag.as_deref()).await {
        // Changed elsewhere, but the user wants it gone.
        Err(DavError::Precondition) => client.delete(&url, None).await,
        other => other,
    };
    match result {
        Ok(()) | Err(DavError::NotFound) => {}
        Err(error) => queue_or_fail(app, &account, error, Pending::Delete { href: href.to_owned() })?,
    }
    calendars.cache.remove(href).map_err(storage)?;
    let _ = app.emit(events::TASKS_CHANGED, ());
    Ok(())
}
