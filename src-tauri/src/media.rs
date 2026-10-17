//! Media sessions (docs/BACKEND.md §4 "Medien"): live from GSMTC events,
//! debounced, sent only when the UI could not have extrapolated it.

use crate::blocking::blocking;
use crate::state::{AppState, Locked};
use rice_core::contract::{MediaAction, MediaSession, MediaState, events};
use rice_core::media::{app_name_fallback, clamp_position, is_meaningful_change};
use rice_core::time::{iso_local, now_unix_ms};
use rice_core::{BackendError, BackendResult};
use rice_services::imaging::{Scaling, bytes_to_jpeg_data_url};
use rice_win::media::{MediaChange, MediaHub, SessionHandle};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const DEBOUNCE: Duration = Duration::from_millis(50);
/// Covers and icons are delivered at most this size (docs/BACKEND.md §2).
const COVER_SIZE: u32 = 300;
const COVER_CACHE: usize = 16;

/// Media service state.
#[derive(Default)]
pub struct Media {
    hub: Mutex<Option<Arc<MediaHub>>>,
    cache: Mutex<Cache>,
    sent: Mutex<Option<MediaState>>,
    /// A debounced refresh is already on its way.
    scheduled: AtomicBool,
    /// Title, artist or cover changed somewhere: re-read them.
    properties_dirty: AtomicBool,
    /// Serialises refreshes (they run on the blocking pool).
    refreshing: Mutex<()>,
}

#[derive(Default)]
struct Cache {
    /// Per session id.
    properties: HashMap<String, Track>,
    /// Cover hash → data URL, most recent last.
    covers: Vec<(u64, Option<String>)>,
    /// AUMID → display name.
    names: HashMap<String, String>,
}

#[derive(Clone)]
struct Track {
    title: String,
    artist: String,
    album: String,
    art_url: Option<String>,
}

/// Connects to GSMTC in the background (the first request blocks).
pub fn start(app: &AppHandle) {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let events = handle.clone();
        match MediaHub::start(move |change| on_change(&events, change)) {
            Ok(hub) => {
                *handle.state::<AppState>().media.hub.locked() = Some(hub);
                schedule(&handle);
            }
            Err(error) => log::warn!("media: session manager unavailable ({error})"),
        }
    });
}

fn on_change(app: &AppHandle, change: MediaChange) {
    if change == MediaChange::Properties {
        app.state::<AppState>().media.properties_dirty.store(true, Ordering::SeqCst);
    }
    schedule(app);
}

/// Refreshes once, `DEBOUNCE` after the first event of a burst.
fn schedule(app: &AppHandle) {
    if app.state::<AppState>().media.scheduled.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(DEBOUNCE).await;
        handle.state::<AppState>().media.scheduled.store(false, Ordering::SeqCst);
        let refresh = handle.clone();
        let _ = tauri::async_runtime::spawn_blocking(move || {
            let state = current(&refresh);
            let media = &refresh.state::<AppState>().media;
            let mut sent = media.sent.locked();
            if sent.as_ref().is_none_or(|previous| is_meaningful_change(previous, &state)) {
                *sent = Some(state.clone());
                drop(sent);
                let _ = refresh.emit(events::MEDIA_STATE, state);
            }
        })
        .await;
    });
}

fn hub(app: &AppHandle) -> Option<Arc<MediaHub>> {
    app.state::<AppState>().media.hub.locked().clone()
}

/// Builds the current state (blocking: reads properties when they changed).
fn current(app: &AppHandle) -> MediaState {
    let Some(hub) = hub(app) else {
        return MediaState::default();
    };
    let media = &app.state::<AppState>().media;
    let _serial = media.refreshing.locked();
    let reread = media.properties_dirty.swap(false, Ordering::SeqCst);
    let (handles, current_id) = hub.sessions();
    let mut cache = media.cache.locked();
    cache.properties.retain(|id, _| handles.iter().any(|h| h.id == *id));
    let sessions = handles.iter().filter_map(|handle| session(handle, reread, &mut cache)).collect();
    MediaState { sessions, current_id }
}

fn session(handle: &SessionHandle, reread: bool, cache: &mut Cache) -> Option<MediaSession> {
    let playback = handle
        .playback()
        .inspect_err(|error| log::debug!("media: {} has no playback info: {error}", handle.aumid))
        .ok()?;
    let track = match cache.properties.get(&handle.id) {
        Some(track) if !reread => track.clone(),
        _ => {
            let track = read_track(handle, cache);
            cache.properties.insert(handle.id.clone(), track.clone());
            track
        }
    };
    let app_name = cache
        .names
        .entry(handle.aumid.clone())
        .or_insert_with(|| rice_win::media::app_display_name(&handle.aumid).unwrap_or_else(|| app_name_fallback(&handle.aumid)))
        .clone();
    let now = now_unix_ms();
    let sampled = playback.updated_unix_ms.filter(|t| *t <= now).unwrap_or(now);
    Some(MediaSession {
        id: handle.id.clone(),
        app_name,
        title: track.title,
        artist: track.artist,
        album: track.album,
        art_url: track.art_url,
        playing: playback.playing,
        position: clamp_position(playback.position, playback.duration),
        position_at: iso_local(sampled),
        duration: playback.duration,
        can_previous: playback.can_previous,
        can_next: playback.can_next,
        can_seek: playback.can_seek,
    })
}

fn read_track(handle: &SessionHandle, cache: &mut Cache) -> Track {
    let properties = match handle.properties() {
        Ok(properties) => properties,
        Err(error) => {
            log::debug!("media: {} has no properties: {error}", handle.aumid);
            Default::default()
        }
    };
    let art_url = properties.cover.as_deref().and_then(|bytes| cover_url(bytes, cache));
    Track { title: properties.title, artist: properties.artist, album: properties.album, art_url }
}

/// Data URL of a cover, cached by content hash.
fn cover_url(bytes: &[u8], cache: &mut Cache) -> Option<String> {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    if let Some(index) = cache.covers.iter().position(|(h, _)| *h == hash) {
        let entry = cache.covers.remove(index);
        let url = entry.1.clone();
        cache.covers.push(entry);
        return url;
    }
    let url = bytes_to_jpeg_data_url(bytes, COVER_SIZE, COVER_SIZE, Scaling::Contain)
        .inspect_err(|error| log::debug!("media: cover not decodable: {error}"))
        .ok();
    cache.covers.push((hash, url.clone()));
    if cache.covers.len() > COVER_CACHE {
        cache.covers.remove(0);
    }
    url
}

/// `media_get`.
pub async fn get(app: &AppHandle) -> BackendResult<MediaState> {
    let handle = app.clone();
    blocking(move || {
        let state = current(&handle);
        *handle.state::<AppState>().media.sent.locked() = Some(state.clone());
        Ok(state)
    })
    .await
}

/// Finds the addressed session; `None` means the current one.
fn find(hub: &MediaHub, id: Option<&str>) -> BackendResult<SessionHandle> {
    let (handles, current) = hub.sessions();
    let wanted = id.map(str::to_owned).or(current);
    let mut handles = handles.into_iter();
    let found = match &wanted {
        Some(id) => handles.find(|h| h.id == *id),
        None => handles.next(),
    };
    found.ok_or_else(|| BackendError::not_found(format!("no media session {}", wanted.unwrap_or_default())))
}

/// `media_control`.
pub async fn control(app: &AppHandle, id: Option<String>, action: MediaAction) -> BackendResult<()> {
    let hub = hub(app).ok_or_else(|| BackendError::not_supported("media sessions unavailable"))?;
    blocking(move || {
        let session = find(&hub, id.as_deref())?;
        if !session.control(action).map_err(BackendError::internal)? {
            log::info!("media: {} refused {action:?}", session.aumid);
        }
        Ok(())
    })
    .await
}

/// `media_seek`.
pub async fn seek(app: &AppHandle, id: Option<String>, position: f64) -> BackendResult<()> {
    if !position.is_finite() {
        return Err(BackendError::invalid_args("position must be a number"));
    }
    let hub = hub(app).ok_or_else(|| BackendError::not_supported("media sessions unavailable"))?;
    blocking(move || {
        let session = find(&hub, id.as_deref())?;
        if !session.seek(position).map_err(BackendError::internal)? {
            log::info!("media: {} refused seeking", session.aumid);
        }
        Ok(())
    })
    .await
}
