//! Media session helpers: ids, app names and which updates are worth sending.

use crate::contract::{MediaSession, MediaState};
use crate::time::parse_unix_ms;

/// How far a timeline update may deviate from the extrapolated position
/// before it counts as a jump (seek, buffering) the UI must hear about.
const POSITION_TOLERANCE: f64 = 1.5;

/// Stable session ids: the app's AUMID, numbered when an app has several sessions.
pub fn session_ids(aumids: &[String]) -> Vec<String> {
    let mut seen: Vec<&str> = Vec::new();
    aumids
        .iter()
        .map(|aumid| {
            let n = seen.iter().filter(|s| **s == aumid.as_str()).count();
            seen.push(aumid);
            if n == 0 { aumid.clone() } else { format!("{aumid}#{}", n + 1) }
        })
        .collect()
}

/// Readable app name when Windows has no display name for the AUMID:
/// `Spotify.exe` → "Spotify", `Microsoft.ZuneMusic_8wekyb3d8bbwe!Music` → "ZuneMusic".
pub fn app_name_fallback(aumid: &str) -> String {
    let base = aumid.rsplit(['\\', '/']).next().unwrap_or(aumid);
    let name = if base.to_ascii_lowercase().ends_with(".exe") {
        &base[..base.len() - 4]
    } else if let Some(package) = base.split('!').next().filter(|_| base.contains('!')) {
        let family = package.split('_').next().unwrap_or(package);
        family.rsplit('.').next().unwrap_or(family)
    } else {
        base
    };
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => aumid.to_owned(),
    }
}

/// Whether `next` differs from `previous` in a way the UI cannot extrapolate.
/// Apps that report their timeline every second would otherwise cause an
/// event per second (docs/BACKEND.md: no periodic sending).
pub fn is_meaningful_change(previous: &MediaState, next: &MediaState) -> bool {
    previous.current_id != next.current_id
        || previous.sessions.len() != next.sessions.len()
        || previous.sessions.iter().zip(&next.sessions).any(|(a, b)| session_changed(a, b))
}

fn session_changed(a: &MediaSession, b: &MediaSession) -> bool {
    let same_except_time = MediaSession { position: b.position, position_at: b.position_at.clone(), ..a.clone() } == *b;
    if !same_except_time {
        return true;
    }
    let expected = if a.playing {
        match (parse_unix_ms(&a.position_at), parse_unix_ms(&b.position_at)) {
            (Some(from), Some(to)) => a.position + (to - from) as f64 / 1000.0,
            _ => return true,
        }
    } else {
        a.position
    };
    (clamp_position(expected, b.duration) - b.position).abs() > POSITION_TOLERANCE
}

/// Clamps a playback position into the track.
pub fn clamp_position(position: f64, duration: f64) -> f64 {
    let position = if position.is_finite() { position.max(0.0) } else { 0.0 };
    if duration > 0.0 { position.min(duration) } else { position }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(playing: bool, position: f64, at: &str) -> MediaSession {
        MediaSession {
            id: "Spotify.exe".into(),
            app_name: "Spotify".into(),
            title: "Song".into(),
            artist: "Band".into(),
            album: "Album".into(),
            art_url: None,
            playing,
            position,
            position_at: at.into(),
            duration: 240.0,
            can_previous: true,
            can_next: true,
            can_seek: true,
        }
    }

    fn state(session: MediaSession) -> MediaState {
        MediaState { current_id: Some(session.id.clone()), sessions: vec![session] }
    }

    #[test]
    fn numbers_duplicate_sessions() {
        let ids = session_ids(&["Chrome".into(), "Spotify.exe".into(), "Chrome".into(), "Chrome".into()]);
        assert_eq!(ids, ["Chrome", "Spotify.exe", "Chrome#2", "Chrome#3"]);
    }

    #[test]
    fn derives_app_names() {
        assert_eq!(app_name_fallback("Spotify.exe"), "Spotify");
        assert_eq!(app_name_fallback("spotify.EXE"), "Spotify");
        assert_eq!(app_name_fallback("C:\\Apps\\foobar2000.exe"), "Foobar2000");
        assert_eq!(app_name_fallback("Microsoft.ZuneMusic_8wekyb3d8bbwe!Microsoft.ZuneMusic"), "ZuneMusic");
        assert_eq!(app_name_fallback("Chrome"), "Chrome");
        assert_eq!(app_name_fallback(""), "");
    }

    #[test]
    fn ignores_timeline_ticks_that_match_the_extrapolation() {
        let a = state(session(true, 10.0, "2026-10-16T12:00:00.000+02:00"));
        let b = state(session(true, 11.0, "2026-10-16T12:00:01.000+02:00"));
        assert!(!is_meaningful_change(&a, &b));
        let paused = state(session(false, 10.0, "2026-10-16T12:00:00.000+02:00"));
        let paused_later = state(session(false, 10.0, "2026-10-16T12:00:05.000+02:00"));
        assert!(!is_meaningful_change(&paused, &paused_later));
    }

    #[test]
    fn reports_seeks_and_real_changes() {
        let a = state(session(true, 10.0, "2026-10-16T12:00:00.000+02:00"));
        let seek = state(session(true, 90.0, "2026-10-16T12:00:01.000+02:00"));
        assert!(is_meaningful_change(&a, &seek));
        let paused = state(session(false, 11.0, "2026-10-16T12:00:01.000+02:00"));
        assert!(is_meaningful_change(&a, &paused));
        let mut other = session(true, 11.0, "2026-10-16T12:00:01.000+02:00");
        other.title = "Next song".into();
        assert!(is_meaningful_change(&a, &state(other)));
        assert!(is_meaningful_change(&a, &MediaState::default()));
    }

    #[test]
    fn clamps_positions() {
        assert_eq!(clamp_position(-3.0, 100.0), 0.0);
        assert_eq!(clamp_position(130.0, 100.0), 100.0);
        assert_eq!(clamp_position(130.0, 0.0), 130.0);
        assert_eq!(clamp_position(f64::NAN, 100.0), 0.0);
    }
}
