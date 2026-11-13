//! Time zones of iCalendar data. Nextcloud, DAVx⁵ and Thunderbird use IANA
//! names; invitations from Outlook use Windows names; old Lightning data
//! prefixes them (`/mozilla.org/20050126_1/Europe/Berlin`). Unknown zones are
//! treated as local time rather than dropping the event.

use crate::ical::Time;
use chrono::{DateTime, Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Windows zone names that appear in invitations, mapped to IANA.
const WINDOWS_ZONES: [(&str, &str); 24] = [
    ("W. Europe Standard Time", "Europe/Berlin"),
    ("Central Europe Standard Time", "Europe/Budapest"),
    ("Central European Standard Time", "Europe/Warsaw"),
    ("Romance Standard Time", "Europe/Paris"),
    ("GMT Standard Time", "Europe/London"),
    ("Greenwich Standard Time", "Atlantic/Reykjavik"),
    ("E. Europe Standard Time", "Europe/Chisinau"),
    ("FLE Standard Time", "Europe/Kyiv"),
    ("GTB Standard Time", "Europe/Bucharest"),
    ("Russian Standard Time", "Europe/Moscow"),
    ("Turkey Standard Time", "Europe/Istanbul"),
    ("Eastern Standard Time", "America/New_York"),
    ("Central Standard Time", "America/Chicago"),
    ("Mountain Standard Time", "America/Denver"),
    ("Pacific Standard Time", "America/Los_Angeles"),
    ("Atlantic Standard Time", "America/Halifax"),
    ("E. South America Standard Time", "America/Sao_Paulo"),
    ("India Standard Time", "Asia/Kolkata"),
    ("China Standard Time", "Asia/Shanghai"),
    ("Tokyo Standard Time", "Asia/Tokyo"),
    ("Singapore Standard Time", "Asia/Singapore"),
    ("AUS Eastern Standard Time", "Australia/Sydney"),
    ("New Zealand Standard Time", "Pacific/Auckland"),
    ("UTC", "UTC"),
];

/// Resolves a TZID to an IANA zone.
pub fn resolve(tzid: &str) -> Option<Tz> {
    let tzid = tzid.trim();
    if let Ok(tz) = tzid.parse::<Tz>() {
        return Some(tz);
    }
    if let Some((_, iana)) = WINDOWS_ZONES.iter().find(|(windows, _)| windows.eq_ignore_ascii_case(tzid)) {
        return iana.parse().ok();
    }
    // "/mozilla.org/20050126_1/Europe/Berlin" → try "Europe/Berlin", then "Berlin".
    let parts: Vec<&str> = tzid.split('/').filter(|p| !p.is_empty()).collect();
    (1..parts.len()).find_map(|skip| parts[skip..].join("/").parse().ok())
}

/// Picks a real instant for a local time, also in DST gaps and overlaps.
fn pick<Z: TimeZone>(result: LocalResult<DateTime<Z>>, local: NaiveDateTime, zone: &Z) -> Option<DateTime<Utc>> {
    match result {
        LocalResult::Single(t) => Some(t.with_timezone(&Utc)),
        LocalResult::Ambiguous(earliest, _) => Some(earliest.with_timezone(&Utc)),
        // A time skipped by the clock change: one hour later exists.
        LocalResult::None => {
            zone.from_local_datetime(&(local + chrono::TimeDelta::hours(1))).earliest().map(|t| t.with_timezone(&Utc))
        }
    }
}

/// The instant of a time; dates mean local midnight.
pub fn to_utc(time: &Time) -> Option<DateTime<Utc>> {
    match time {
        Time::Utc(t) => Some(t.and_utc()),
        Time::Date(d) => {
            let midnight = d.and_hms_opt(0, 0, 0)?;
            pick(Local.from_local_datetime(&midnight), midnight, &Local)
        }
        Time::Floating(t) => pick(Local.from_local_datetime(t), *t, &Local),
        Time::Zoned(t, tzid) => match resolve(tzid) {
            Some(tz) => pick(tz.from_local_datetime(t), *t, &tz),
            None => pick(Local.from_local_datetime(t), *t, &Local),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn naive(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap()
    }

    #[test]
    fn resolves_iana_windows_and_prefixed_names() {
        assert_eq!(resolve("Europe/Berlin"), Some(chrono_tz::Europe::Berlin));
        assert_eq!(resolve("W. Europe Standard Time"), Some(chrono_tz::Europe::Berlin));
        assert_eq!(resolve("/mozilla.org/20050126_1/Europe/Berlin"), Some(chrono_tz::Europe::Berlin));
        assert_eq!(resolve("Nowhere Standard Time"), None);
    }

    #[test]
    fn converts_zoned_times_across_dst() {
        let summer = Time::Zoned(naive("2026-07-01 10:00"), "Europe/Berlin".into());
        assert_eq!(to_utc(&summer).unwrap().to_rfc3339(), "2026-07-01T08:00:00+00:00");
        let winter = Time::Zoned(naive("2026-12-01 10:00"), "Europe/Berlin".into());
        assert_eq!(to_utc(&winter).unwrap().to_rfc3339(), "2026-12-01T09:00:00+00:00");
        // 02:30 does not exist on 2026-03-29 in Berlin.
        let gap = Time::Zoned(naive("2026-03-29 02:30"), "Europe/Berlin".into());
        assert_eq!(to_utc(&gap).unwrap().to_rfc3339(), "2026-03-29T01:30:00+00:00");
        assert_eq!(to_utc(&Time::Utc(naive("2026-07-01 10:00"))).unwrap().to_rfc3339(), "2026-07-01T10:00:00+00:00");
    }

    #[test]
    fn dates_are_local_midnight() {
        let date = Time::Date(NaiveDate::from_ymd_opt(2026, 10, 20).unwrap());
        let utc = to_utc(&date).unwrap();
        assert_eq!(utc.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string(), "2026-10-20 00:00");
    }
}
