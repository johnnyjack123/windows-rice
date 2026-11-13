//! Event occurrences of a calendar object: recurrences expanded with
//! `rrule`, exceptions (EXDATE), moved instances (RECURRENCE-ID) and
//! cancellations applied.

use crate::ical::{Component, Property, Time, format_date, format_local, format_utc, parse_duration, parse_time, parse_times};
use crate::zones::{resolve, to_utc};
use chrono::{DateTime, Local, TimeDelta, Utc};
use rrule::RRuleSet;
use std::collections::HashMap;

/// Most occurrences one series may produce per query.
const MAX_OCCURRENCES: u16 = 1000;

/// One occurrence of an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub uid: String,
    /// Identifies the occurrence within its series (UTC instant or date).
    pub recurrence: String,
    pub title: String,
    pub location: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
}

impl Occurrence {
    /// The contract id: `${uid}@${recurrenceId}`.
    pub fn id(&self) -> String {
        format!("{}@{}", self.uid, self.recurrence)
    }
}

/// Key of an occurrence: the date for all-day events, else the UTC instant.
fn recurrence_key(start: &Time) -> Option<String> {
    match start {
        Time::Date(date) => Some(format_date(*date)),
        other => to_utc(other).map(format_utc),
    }
}

fn is_cancelled(event: &Component) -> bool {
    event.property("STATUS").is_some_and(|s| s.value.eq_ignore_ascii_case("CANCELLED"))
}

/// Start, length and all-day flag of an event component.
fn timing(event: &Component) -> Option<(Time, DateTime<Utc>, TimeDelta)> {
    let start = parse_time(event.property("DTSTART")?)?;
    let start_utc = to_utc(&start)?;
    let duration = if let Some(end) = event.property("DTEND").and_then(parse_time) {
        match (&start, &end) {
            (Time::Date(a), Time::Date(b)) => TimeDelta::try_days((*b - *a).num_days()).unwrap_or_default(),
            _ => to_utc(&end).map_or(TimeDelta::zero(), |e| e - start_utc),
        }
    } else if let Some(duration) = event.property("DURATION").and_then(|d| parse_duration(&d.value)) {
        duration
    } else if start.is_date() {
        TimeDelta::days(1)
    } else {
        TimeDelta::zero()
    };
    Some((start, start_utc, duration.max(TimeDelta::zero())))
}

/// How a time is written for `rrule`, following the series' DTSTART.
#[derive(Debug, Clone)]
enum Frame {
    Date,
    Utc,
    Floating,
    Zone(chrono_tz::Tz),
}

impl Frame {
    fn of(start: &Time) -> Self {
        match start {
            Time::Date(_) => Self::Date,
            Time::Utc(_) => Self::Utc,
            Time::Floating(_) => Self::Floating,
            Time::Zoned(_, tzid) => resolve(tzid).map_or(Self::Floating, Self::Zone),
        }
    }

    /// `NAME;params:value` for one instant in this frame.
    fn line(&self, name: &str, instant: DateTime<Utc>, date: Option<chrono::NaiveDate>) -> String {
        match self {
            Self::Date => {
                let date = date.unwrap_or_else(|| instant.with_timezone(&Local).date_naive());
                format!("{name};VALUE=DATE:{}", format_date(date))
            }
            Self::Utc => format!("{name}:{}", format_utc(instant)),
            Self::Floating => format!("{name}:{}", format_local(instant.with_timezone(&Local).naive_local())),
            Self::Zone(tz) => format!("{name};TZID={}:{}", tz.name(), format_local(instant.with_timezone(tz).naive_local())),
        }
    }
}

/// Rewrites UNTIL to match DTSTART, as RFC 5545 requires and `rrule` checks:
/// a date for all-day series, UTC for zoned ones, floating for floating ones.
fn normalise_rule(rule: &str, frame: &Frame) -> String {
    rule.split(';')
        .map(|part| {
            let Some(value) = part.strip_prefix("UNTIL=").or_else(|| part.strip_prefix("until=")) else {
                return part.to_owned();
            };
            let parsed = crate::ical::parse_time_value(value, false, None);
            let instant = parsed.as_ref().and_then(to_utc);
            match (frame, parsed, instant) {
                (Frame::Date, Some(Time::Date(d)), _) => format!("UNTIL={}", format_date(d)),
                (Frame::Date, _, Some(i)) => format!("UNTIL={}", format_date(i.with_timezone(&Local).date_naive())),
                (Frame::Floating, _, Some(i)) => format!("UNTIL={}", format_local(i.with_timezone(&Local).naive_local())),
                (Frame::Utc | Frame::Zone(_), _, Some(i)) => format!("UNTIL={}", format_utc(i)),
                _ => part.to_owned(),
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Builds the `rrule` input of a series.
fn rule_set(master: &Component, start: &Time, start_utc: DateTime<Utc>) -> Option<RRuleSet> {
    let frame = Frame::of(start);
    let start_date = match start {
        Time::Date(d) => Some(*d),
        _ => None,
    };
    let mut lines = vec![frame.line("DTSTART", start_utc, start_date)];
    for rule in master.properties_named("RRULE") {
        lines.push(format!("RRULE:{}", normalise_rule(&rule.value, &frame)));
    }
    let mut add = |name: &str, property: &Property| {
        for time in parse_times(property) {
            let date = match &time {
                Time::Date(d) => Some(*d),
                _ => None,
            };
            if let Some(instant) = to_utc(&time) {
                lines.push(frame.line(name, instant, date));
            }
        }
    };
    for exdate in master.properties_named("EXDATE") {
        add("EXDATE", exdate);
    }
    for rdate in master.properties_named("RDATE").filter(|p| p.param("VALUE").is_none_or(|v| !v.eq_ignore_ascii_case("PERIOD"))) {
        add("RDATE", rdate);
    }
    match lines.join("\n").parse::<RRuleSet>() {
        Ok(set) => Some(set),
        Err(error) => {
            log::debug!("calendar: recurrence not understood ({error}); showing the first occurrence only");
            None
        }
    }
}

fn is_recurring(event: &Component) -> bool {
    event.property("RRULE").is_some() || event.property("RDATE").is_some()
}

/// All occurrences of the calendar object's events that overlap `[from, to)`.
pub fn occurrences(calendar: &Component, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<Occurrence> {
    let mut by_uid: HashMap<String, (Option<&Component>, Vec<&Component>)> = HashMap::new();
    for event in calendar.children_named("VEVENT") {
        let uid = event.text("UID").unwrap_or_default();
        let entry = by_uid.entry(uid).or_default();
        if event.property("RECURRENCE-ID").is_some() {
            entry.1.push(event);
        } else if entry.0.is_none() {
            entry.0 = Some(event);
        }
    }
    let overlaps = |o: &Occurrence| if o.end > o.start { o.end > from && o.start < to } else { o.start >= from && o.start < to };
    let mut out = Vec::new();
    for (uid, (master, overrides)) in by_uid {
        let moved: Vec<String> = overrides
            .iter()
            .filter_map(|o| o.property("RECURRENCE-ID").and_then(parse_time).and_then(|t| recurrence_key(&t)))
            .collect();
        if let Some(master) = master.filter(|m| !is_cancelled(m))
            && let Some((start, start_utc, duration)) = timing(master)
        {
            let all_day = start.is_date();
            let title = master.text("SUMMARY").unwrap_or_default();
            let location = master.text("LOCATION").unwrap_or_default();
            let starts: Vec<DateTime<Utc>> = match is_recurring(master).then(|| rule_set(master, &start, start_utc)).flatten() {
                Some(set) => {
                    let window_start = (from - duration - TimeDelta::days(1)).with_timezone(&rrule::Tz::UTC);
                    let window_end = to.with_timezone(&rrule::Tz::UTC);
                    set.after(window_start).before(window_end).all(MAX_OCCURRENCES).dates.iter().map(|d| d.with_timezone(&Utc)).collect()
                }
                None => vec![start_utc],
            };
            for occurrence_start in starts {
                let key = if all_day {
                    format_date(occurrence_start.with_timezone(&Local).date_naive())
                } else {
                    format_utc(occurrence_start)
                };
                if moved.contains(&key) {
                    continue;
                }
                let occurrence = Occurrence {
                    uid: uid.clone(),
                    recurrence: key,
                    title: title.clone(),
                    location: location.clone(),
                    start: occurrence_start,
                    end: occurrence_start + duration,
                    all_day,
                };
                if overlaps(&occurrence) {
                    out.push(occurrence);
                }
            }
        }
        for moved_event in overrides.into_iter().filter(|o| !is_cancelled(o)) {
            let Some((start, start_utc, duration)) = timing(moved_event) else { continue };
            let Some(key) = moved_event.property("RECURRENCE-ID").and_then(parse_time).and_then(|t| recurrence_key(&t)) else {
                continue;
            };
            let occurrence = Occurrence {
                uid: uid.clone(),
                recurrence: key,
                title: moved_event.text("SUMMARY").or_else(|| master.and_then(|m| m.text("SUMMARY"))).unwrap_or_default(),
                location: moved_event.text("LOCATION").or_else(|| master.and_then(|m| m.text("LOCATION"))).unwrap_or_default(),
                start: start_utc,
                end: start_utc + duration,
                all_day: start.is_date(),
            };
            if overlaps(&occurrence) {
                out.push(occurrence);
            }
        }
    }
    out.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.title.cmp(&b.title)));
    out
}

/// Time span of a calendar object for pre-filtering in the cache:
/// (first start, last end); the end is `None` for recurring series.
pub fn bounds(calendar: &Component) -> (Option<i64>, Option<i64>) {
    let mut first: Option<i64> = None;
    let mut last: Option<i64> = None;
    let mut recurring = false;
    for event in calendar.children_named("VEVENT") {
        let Some((_, start, duration)) = timing(event) else { continue };
        recurring |= is_recurring(event);
        first = Some(first.map_or(start.timestamp(), |f| f.min(start.timestamp())));
        let end = (start + duration).timestamp();
        last = Some(last.map_or(end, |l| l.max(end)));
    }
    (first, if recurring { None } else { last })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn calendar(body: &str) -> Component {
        Component::parse(&format!("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n{body}END:VCALENDAR\r\n")).unwrap()
    }

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn expands_weekly_series_with_exceptions_and_moves() {
        let cal = calendar(
            "BEGIN:VEVENT\r\nUID:standup\r\nSUMMARY:Standup\r\nLOCATION:Teams\r\n\
DTSTART;TZID=Europe/Berlin:20261005T093000\r\nDTEND;TZID=Europe/Berlin:20261005T094500\r\n\
RRULE:FREQ=WEEKLY;BYDAY=MO;UNTIL=20261102T083000Z\r\nEXDATE;TZID=Europe/Berlin:20261012T093000\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:standup\r\nRECURRENCE-ID;TZID=Europe/Berlin:20261019T093000\r\nSUMMARY:Standup (spät)\r\n\
DTSTART;TZID=Europe/Berlin:20261019T110000\r\nDTEND;TZID=Europe/Berlin:20261019T111500\r\nEND:VEVENT\r\n",
        );
        let list = occurrences(&cal, utc("2026-10-01T00:00:00Z"), utc("2026-11-30T00:00:00Z"));
        let summary: Vec<_> = list.iter().map(|o| (o.start.to_rfc3339(), o.title.as_str())).collect();
        assert_eq!(
            summary,
            [
                ("2026-10-05T07:30:00+00:00".to_owned(), "Standup"),
                ("2026-10-19T09:00:00+00:00".to_owned(), "Standup (spät)"),
                // After the switch to winter time 09:30 Berlin is 08:30 UTC.
                ("2026-10-26T08:30:00+00:00".to_owned(), "Standup"),
                ("2026-11-02T08:30:00+00:00".to_owned(), "Standup"),
            ]
        );
        assert!(list.iter().all(|o| o.location == "Teams" && !o.all_day));
        assert_eq!(list[0].end - list[0].start, TimeDelta::minutes(15));
        assert_eq!(list[1].id(), "standup@20261019T073000Z");
    }

    #[test]
    fn handles_all_day_and_multi_day_events() {
        let cal = calendar(
            "BEGIN:VEVENT\r\nUID:holiday\r\nSUMMARY:Urlaub\r\nDTSTART;VALUE=DATE:20261020\r\nDTEND;VALUE=DATE:20261023\r\nEND:VEVENT\r\n",
        );
        let list = occurrences(&cal, utc("2026-10-21T00:00:00Z"), utc("2026-10-22T00:00:00Z"));
        assert_eq!(list.len(), 1);
        let o = &list[0];
        assert!(o.all_day);
        assert_eq!(o.start.with_timezone(&Local).date_naive(), NaiveDate::from_ymd_opt(2026, 10, 20).unwrap());
        assert_eq!((o.end - o.start).num_days(), 3);
        assert_eq!(o.id(), "holiday@20261020");
    }

    #[test]
    fn yearly_birthdays_with_date_until() {
        let cal = calendar(
            "BEGIN:VEVENT\r\nUID:bday\r\nSUMMARY:Geburtstag\r\nDTSTART;VALUE=DATE:19900315\r\nRRULE:FREQ=YEARLY;UNTIL=20280315T000000Z\r\nEND:VEVENT\r\n",
        );
        let list = occurrences(&cal, utc("2026-01-01T00:00:00Z"), utc("2030-01-01T00:00:00Z"));
        let years: Vec<_> = list.iter().map(|o| o.start.with_timezone(&Local).format("%Y-%m-%d").to_string()).collect();
        assert_eq!(years, ["2026-03-15", "2027-03-15", "2028-03-15"]);
    }

    #[test]
    fn skips_cancelled_and_keeps_long_running_events() {
        let cal = calendar(
            "BEGIN:VEVENT\r\nUID:a\r\nSTATUS:CANCELLED\r\nDTSTART:20261020T080000Z\r\nDURATION:PT1H\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:b\r\nSUMMARY:Konferenz\r\nDTSTART:20261019T080000Z\r\nDURATION:P3D\r\nEND:VEVENT\r\n",
        );
        let list = occurrences(&cal, utc("2026-10-20T00:00:00Z"), utc("2026-10-21T00:00:00Z"));
        assert_eq!(list.iter().map(|o| o.uid.as_str()).collect::<Vec<_>>(), ["b"]);
    }

    #[test]
    fn falls_back_to_the_first_occurrence_for_broken_rules() {
        let cal = calendar(
            "BEGIN:VEVENT\r\nUID:x\r\nSUMMARY:X\r\nDTSTART:20261020T080000Z\r\nRRULE:FREQ=SOMETIMES\r\nEND:VEVENT\r\n",
        );
        assert_eq!(occurrences(&cal, utc("2026-10-01T00:00:00Z"), utc("2026-11-01T00:00:00Z")).len(), 1);
    }

    #[test]
    fn computes_bounds() {
        let single = calendar("BEGIN:VEVENT\r\nUID:s\r\nDTSTART:20261020T080000Z\r\nDTEND:20261020T090000Z\r\nEND:VEVENT\r\n");
        assert_eq!(bounds(&single), (Some(utc("2026-10-20T08:00:00Z").timestamp()), Some(utc("2026-10-20T09:00:00Z").timestamp())));
        let series = calendar("BEGIN:VEVENT\r\nUID:r\r\nDTSTART:20261020T080000Z\r\nRRULE:FREQ=DAILY\r\nEND:VEVENT\r\n");
        assert_eq!(bounds(&series).1, None);
        assert_eq!(bounds(&calendar("")), (None, None));
    }
}
