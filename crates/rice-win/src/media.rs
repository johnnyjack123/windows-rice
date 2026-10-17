//! Media sessions of other apps via `GlobalSystemMediaTransportControls`
//! (Spotify, browsers, the Media Player …).
//!
//! WinRT async calls are awaited with `join()`, which blocks: call the query
//! and control functions from worker threads, never from the UI thread.

use rice_core::contract::MediaAction;
use rice_core::media::session_ids;
use rice_core::time::windows_ticks_to_unix_ms;
use std::sync::{Arc, Mutex, PoisonError, Weak};
use windows::ApplicationModel::AppInfo;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::{Buffer, DataReader, IRandomAccessStreamReference, InputStreamOptions};
use windows::core::{HSTRING, RuntimeType};

/// Covers larger than this are ignored (a cover is a few hundred KB at most).
const MAX_COVER_BYTES: u32 = 8 * 1024 * 1024;
const TICKS_PER_SECOND: f64 = 10_000_000.0;

/// What changed. Property changes require re-reading title, artist and cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaChange {
    Properties,
    Playback,
}

type Notify = Arc<dyn Fn(MediaChange) + Send + Sync>;

/// The session manager plus event subscriptions on every session.
pub struct MediaHub {
    manager: Manager,
    notify: Notify,
    subscriptions: Mutex<Vec<Subscription>>,
}

struct Subscription {
    session: Session,
    tokens: [i64; 3],
}

/// Playback state of one session.
#[derive(Debug, Clone, PartialEq)]
pub struct Playback {
    pub playing: bool,
    pub can_previous: bool,
    pub can_next: bool,
    pub can_seek: bool,
    /// Seconds from the start of the track.
    pub position: f64,
    pub duration: f64,
    /// When the app last reported the position.
    pub updated_unix_ms: Option<i64>,
}

/// Track metadata of one session.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Properties {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Raw cover image bytes (any format).
    pub cover: Option<Vec<u8>>,
}

/// One session with its stable id.
pub struct SessionHandle {
    pub id: String,
    pub aumid: String,
    session: Session,
}

impl MediaHub {
    /// Connects to the session manager and reports every change through `notify`.
    pub fn start(notify: impl Fn(MediaChange) + Send + Sync + 'static) -> windows::core::Result<Arc<Self>> {
        let manager = Manager::RequestAsync()?.join()?;
        let hub = Arc::new(Self { manager, notify: Arc::new(notify), subscriptions: Mutex::new(Vec::new()) });
        let weak: Weak<Self> = Arc::downgrade(&hub);
        hub.manager.SessionsChanged(&TypedEventHandler::new(move |_, _| {
            if let Some(hub) = weak.upgrade() {
                hub.subscribe_sessions();
                (hub.notify)(MediaChange::Properties);
            }
            Ok(())
        }))?;
        let notify = Arc::clone(&hub.notify);
        hub.manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            notify(MediaChange::Playback);
            Ok(())
        }))?;
        hub.subscribe_sessions();
        Ok(hub)
    }

    /// Re-subscribes to the property, playback and timeline events of every session.
    fn subscribe_sessions(&self) {
        let mut subscriptions = self.subscriptions.lock().unwrap_or_else(PoisonError::into_inner);
        for old in subscriptions.drain(..) {
            let _ = old.session.RemoveMediaPropertiesChanged(old.tokens[0]);
            let _ = old.session.RemovePlaybackInfoChanged(old.tokens[1]);
            let _ = old.session.RemoveTimelinePropertiesChanged(old.tokens[2]);
        }
        for session in self.raw_sessions() {
            let tokens = (|| {
                Ok::<_, windows::core::Error>([
                    session.MediaPropertiesChanged(&handler(&self.notify, MediaChange::Properties))?,
                    session.PlaybackInfoChanged(&handler(&self.notify, MediaChange::Playback))?,
                    session.TimelinePropertiesChanged(&handler(&self.notify, MediaChange::Playback))?,
                ])
            })();
            match tokens {
                Ok(tokens) => subscriptions.push(Subscription { session, tokens }),
                Err(error) => log::warn!("media: cannot watch a session: {error}"),
            }
        }
    }

    fn raw_sessions(&self) -> Vec<Session> {
        let Ok(list) = self.manager.GetSessions() else {
            return Vec::new();
        };
        let count = list.Size().unwrap_or(0);
        (0..count).filter_map(|i| list.GetAt(i).ok()).collect()
    }

    /// All sessions with stable ids, plus the id of the current one.
    pub fn sessions(&self) -> (Vec<SessionHandle>, Option<String>) {
        let sessions = self.raw_sessions();
        let aumids: Vec<String> =
            sessions.iter().map(|s| s.SourceAppUserModelId().map(|a| a.to_string()).unwrap_or_default()).collect();
        let ids = session_ids(&aumids);
        let current = self.manager.GetCurrentSession().ok();
        let current_id = current.and_then(|c| {
            sessions
                .iter()
                .position(|s| *s == c)
                .or_else(|| {
                    let aumid = c.SourceAppUserModelId().ok()?.to_string();
                    aumids.iter().position(|a| *a == aumid)
                })
                .map(|i| ids[i].clone())
        });
        let handles = sessions
            .into_iter()
            .zip(aumids)
            .zip(ids)
            .map(|((session, aumid), id)| SessionHandle { id, aumid, session })
            .collect();
        (handles, current_id)
    }
}

impl SessionHandle {
    /// Play state, controls and timeline (fast, no async calls).
    pub fn playback(&self) -> windows::core::Result<Playback> {
        let info = self.session.GetPlaybackInfo()?;
        let controls = info.Controls()?;
        let timeline = self.session.GetTimelineProperties()?;
        let start = timeline.StartTime()?.Duration;
        let end = timeline.EndTime()?.Duration;
        let position = timeline.Position()?.Duration;
        Ok(Playback {
            playing: info.PlaybackStatus()? == Status::Playing,
            can_previous: controls.IsPreviousEnabled()?,
            can_next: controls.IsNextEnabled()?,
            can_seek: controls.IsPlaybackPositionEnabled()?,
            position: (position - start).max(0) as f64 / TICKS_PER_SECOND,
            duration: (end - start).max(0) as f64 / TICKS_PER_SECOND,
            updated_unix_ms: windows_ticks_to_unix_ms(timeline.LastUpdatedTime()?.UniversalTime),
        })
    }

    /// Title, artist, album and cover (blocks while the app answers).
    pub fn properties(&self) -> windows::core::Result<Properties> {
        let props = self.session.TryGetMediaPropertiesAsync()?.join()?;
        let cover = props.Thumbnail().ok().and_then(|reference| match read_all(&reference) {
            Ok(bytes) if !bytes.is_empty() => Some(bytes),
            Ok(_) => None,
            Err(error) => {
                log::debug!("media: cover of {} unreadable: {error}", self.aumid);
                None
            }
        });
        Ok(Properties {
            title: props.Title()?.to_string(),
            artist: props.Artist()?.to_string(),
            album: props.AlbumTitle()?.to_string(),
            cover,
        })
    }

    /// Play, pause, toggle, next or previous. `false` if the app refused.
    pub fn control(&self, action: MediaAction) -> windows::core::Result<bool> {
        let operation = match action {
            MediaAction::Play => self.session.TryPlayAsync()?,
            MediaAction::Pause => self.session.TryPauseAsync()?,
            MediaAction::Toggle => self.session.TryTogglePlayPauseAsync()?,
            MediaAction::Next => self.session.TrySkipNextAsync()?,
            MediaAction::Previous => self.session.TrySkipPreviousAsync()?,
        };
        operation.join()
    }

    /// Jumps to `seconds` from the start of the track.
    pub fn seek(&self, seconds: f64) -> windows::core::Result<bool> {
        let start = self.session.GetTimelineProperties()?.StartTime()?.Duration;
        let ticks = start + (seconds.max(0.0) * TICKS_PER_SECOND) as i64;
        self.session.TryChangePlaybackPositionAsync(ticks)?.join()
    }
}

/// Event handler that only reports which kind of change happened.
fn handler<A: RuntimeType + 'static>(notify: &Notify, change: MediaChange) -> TypedEventHandler<Session, A> {
    let notify = Arc::clone(notify);
    TypedEventHandler::new(move |_, _| {
        notify(change);
        Ok(())
    })
}

/// Display name of an app (Start menu name), if Windows knows the AUMID.
pub fn app_display_name(aumid: &str) -> Option<String> {
    let info = AppInfo::GetFromAppUserModelId(&HSTRING::from(aumid)).ok()?;
    let name = info.DisplayInfo().ok()?.DisplayName().ok()?.to_string();
    (!name.is_empty()).then_some(name)
}

fn read_all(reference: &IRandomAccessStreamReference) -> windows::core::Result<Vec<u8>> {
    let stream = reference.OpenReadAsync()?.join()?;
    let size = u32::try_from(stream.Size()?).unwrap_or(u32::MAX);
    if size == 0 || size > MAX_COVER_BYTES {
        return Ok(Vec::new());
    }
    let buffer = Buffer::Create(size)?;
    let filled = stream.ReadAsync(&buffer, size, InputStreamOptions::None)?.join()?;
    let mut bytes = vec![0u8; filled.Length()? as usize];
    DataReader::FromBuffer(&filled)?.ReadBytes(&mut bytes)?;
    Ok(bytes)
}
