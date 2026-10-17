//! Timestamps on the wire: ISO 8601 with the local offset (docs/BACKEND.md §2).

use chrono::{DateTime, Local, SecondsFormat, TimeZone};

/// 100-ns ticks between 1601-01-01 (Windows epoch) and 1970-01-01.
const EPOCH_DIFFERENCE_TICKS: i64 = 116_444_736_000_000_000;

/// Converts a Windows `DateTime`/`FILETIME` (100-ns ticks since 1601) to Unix ms.
/// Zero or pre-1970 values (unset timestamps) give `None`.
pub fn windows_ticks_to_unix_ms(ticks: i64) -> Option<i64> {
    (ticks > EPOCH_DIFFERENCE_TICKS).then(|| (ticks - EPOCH_DIFFERENCE_TICKS) / 10_000)
}

/// Unix ms as ISO 8601 with the local offset, e.g. "2026-10-16T12:00:00.000+02:00".
pub fn iso_local(unix_ms: i64) -> String {
    match Local.timestamp_millis_opt(unix_ms).single() {
        Some(time) => time.to_rfc3339_opts(SecondsFormat::Millis, false),
        None => now_iso(),
    }
}

/// The current time as ISO 8601 with the local offset.
pub fn now_iso() -> String {
    Local::now().to_rfc3339_opts(SecondsFormat::Millis, false)
}

/// Current Unix time in ms.
pub fn now_unix_ms() -> i64 {
    Local::now().timestamp_millis()
}

/// Parses an ISO 8601 timestamp back to Unix ms.
pub fn parse_unix_ms(iso: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(iso).ok().map(|t| t.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_windows_ticks() {
        // 2026-01-01T00:00:00Z
        let unix_ms = 1_767_225_600_000;
        assert_eq!(windows_ticks_to_unix_ms(unix_ms * 10_000 + EPOCH_DIFFERENCE_TICKS), Some(unix_ms));
        assert_eq!(windows_ticks_to_unix_ms(0), None);
    }

    #[test]
    fn formats_with_offset_and_parses_back() {
        let iso = iso_local(1_767_225_600_123);
        assert!(iso.contains('T'));
        assert!(iso.ends_with('Z') || iso[iso.len() - 6..].starts_with(['+', '-']), "{iso}");
        assert_eq!(parse_unix_ms(&iso), Some(1_767_225_600_123));
        assert_eq!(parse_unix_ms("yesterday"), None);
    }
}
