//! VTODO tasks: reading, creating and patching. Patches edit the stored text
//! in place, so properties of other clients survive.

use crate::ical::{Component, Property, Time, format_date, format_utc, parse_time};
use crate::zones::to_utc;
use chrono::{DateTime, NaiveDate, Utc};
use rice_core::contract::{Task, TaskPatch};
use rice_core::time::iso_local;

const PRODID: &str = "-//rice//rice desktop//EN";

/// The task in a calendar object, if it holds a VTODO.
pub fn task(calendar: &Component, id: &str, calendar_id: &str) -> Option<Task> {
    let todo = calendar.children_named("VTODO").next()?;
    let due = todo.property("DUE").and_then(parse_time).and_then(|due| match due {
        Time::Date(date) => Some(date.format("%Y-%m-%d").to_string()),
        other => to_utc(&other).map(|t| iso_local(t.timestamp_millis())),
    });
    let done = todo.property("STATUS").is_some_and(|s| s.value.eq_ignore_ascii_case("COMPLETED"));
    let completed_at = todo
        .property("COMPLETED")
        .and_then(parse_time)
        .and_then(|t| to_utc(&t))
        .map(|t| iso_local(t.timestamp_millis()));
    Some(Task {
        id: id.to_owned(),
        calendar_id: calendar_id.to_owned(),
        title: todo.text("SUMMARY").unwrap_or_default(),
        due,
        done,
        completed_at,
    })
}

/// DUE property for "YYYY-MM-DD" (a date) or an ISO date-time (stored in UTC).
fn due_property(due: &str) -> Option<Property> {
    if let Ok(date) = NaiveDate::parse_from_str(due, "%Y-%m-%d") {
        return Some(Property::new("DUE", format_date(date)).with_param("VALUE", "DATE"));
    }
    let instant = DateTime::parse_from_rfc3339(due).ok()?.with_timezone(&Utc);
    Some(Property::new("DUE", format_utc(instant)))
}

/// A new calendar object with one open task.
pub fn new_task(uid: &str, title: &str, due: Option<&str>, now: DateTime<Utc>) -> Result<String, String> {
    let stamp = format_utc(now);
    let mut todo = Component::new("VTODO");
    todo.properties.push(Property::new("UID", uid));
    todo.properties.push(Property::new("DTSTAMP", stamp.clone()));
    todo.properties.push(Property::new("CREATED", stamp.clone()));
    todo.properties.push(Property::new("LAST-MODIFIED", stamp));
    todo.properties.push(Property::text("SUMMARY", title));
    if let Some(due) = due {
        todo.properties.push(due_property(due).ok_or_else(|| format!("invalid due date {due:?}"))?);
    }
    todo.properties.push(Property::new("STATUS", "NEEDS-ACTION"));
    let mut calendar = Component::new("VCALENDAR");
    calendar.properties.push(Property::new("VERSION", "2.0"));
    calendar.properties.push(Property::new("PRODID", PRODID));
    calendar.children.push(todo);
    Ok(calendar.to_ics())
}

/// Applies a patch to a stored task and returns the new text.
pub fn apply_patch(ics: &str, patch: &TaskPatch, now: DateTime<Utc>) -> Result<String, String> {
    let mut calendar = Component::parse(ics)?;
    let todo = calendar.child_mut("VTODO").ok_or("no VTODO in this object")?;
    if let Some(title) = &patch.title {
        todo.set(Property::text("SUMMARY", title));
    }
    match &patch.due {
        Some(Some(due)) => todo.set(due_property(due).ok_or_else(|| format!("invalid due date {due:?}"))?),
        Some(None) => todo.remove("DUE"),
        None => {}
    }
    match patch.done {
        Some(true) => {
            todo.set(Property::new("STATUS", "COMPLETED"));
            todo.set(Property::new("COMPLETED", format_utc(now)));
            todo.set(Property::new("PERCENT-COMPLETE", "100"));
        }
        Some(false) => {
            todo.set(Property::new("STATUS", "NEEDS-ACTION"));
            todo.remove("COMPLETED");
            todo.remove("PERCENT-COMPLETE");
        }
        None => {}
    }
    let stamp = format_utc(now);
    todo.set(Property::new("LAST-MODIFIED", stamp.clone()));
    todo.set(Property::new("DTSTAMP", stamp));
    if let Some(sequence) = todo.property("SEQUENCE").and_then(|s| s.value.trim().parse::<u32>().ok()) {
        todo.set(Property::new("SEQUENCE", (sequence + 1).to_string()));
    }
    Ok(calendar.to_ics())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-17T12:00:00Z").unwrap().with_timezone(&Utc)
    }

    const FROM_TASKS_ORG: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:+//IDN tasks.org//android-130804//EN\r\n\
BEGIN:VTODO\r\nDTSTAMP:20261001T080000Z\r\nUID:6871830163932520932\r\nCREATED:20261001T080000Z\r\nLAST-MODIFIED:20261001T080000Z\r\n\
SUMMARY:Steuer\\, endlich\r\nPRIORITY:9\r\nX-APPLE-SORT-ORDER:-123\r\nDUE;VALUE=DATE:20261031\r\nSEQUENCE:2\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    #[test]
    fn reads_tasks() {
        let calendar = Component::parse(FROM_TASKS_ORG).unwrap();
        let task = task(&calendar, "/t.ics", "c").unwrap();
        assert_eq!(task.title, "Steuer, endlich");
        assert_eq!(task.due.as_deref(), Some("2026-10-31"));
        assert!(!task.done);
        assert_eq!(task.completed_at, None);
        let event = Component::parse("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:e\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n").unwrap();
        assert!(super::task(&event, "x", "c").is_none());
    }

    #[test]
    fn completes_and_reopens_keeping_foreign_properties() {
        let done = apply_patch(FROM_TASKS_ORG, &TaskPatch { done: Some(true), ..Default::default() }, now()).unwrap();
        let calendar = Component::parse(&done).unwrap();
        let todo = calendar.children_named("VTODO").next().unwrap();
        assert_eq!(todo.property("STATUS").unwrap().value, "COMPLETED");
        assert_eq!(todo.property("COMPLETED").unwrap().value, "20261017T120000Z");
        assert_eq!(todo.property("PERCENT-COMPLETE").unwrap().value, "100");
        assert_eq!(todo.property("SEQUENCE").unwrap().value, "3");
        assert_eq!(todo.property("X-APPLE-SORT-ORDER").unwrap().value, "-123");
        assert_eq!(todo.property("PRIORITY").unwrap().value, "9");
        let task = task(&calendar, "/t.ics", "c").unwrap();
        assert!(task.done);
        assert!(task.completed_at.is_some());

        let reopened = apply_patch(&done, &TaskPatch { done: Some(false), ..Default::default() }, now()).unwrap();
        let calendar = Component::parse(&reopened).unwrap();
        let todo = calendar.children_named("VTODO").next().unwrap();
        assert_eq!(todo.property("STATUS").unwrap().value, "NEEDS-ACTION");
        assert!(todo.property("COMPLETED").is_none() && todo.property("PERCENT-COMPLETE").is_none());
    }

    #[test]
    fn changes_title_and_due() {
        let patch = TaskPatch { title: Some("Neu".into()), due: Some(Some("2026-11-02".into())), done: None };
        let changed = apply_patch(FROM_TASKS_ORG, &patch, now()).unwrap();
        let task = task(&Component::parse(&changed).unwrap(), "i", "c").unwrap();
        assert_eq!((task.title.as_str(), task.due.as_deref()), ("Neu", Some("2026-11-02")));
        let cleared = apply_patch(&changed, &TaskPatch { due: Some(None), ..Default::default() }, now()).unwrap();
        assert_eq!(task_of(&cleared).due, None);
        let timed = apply_patch(&changed, &TaskPatch { due: Some(Some("2026-11-02T15:00:00+01:00".into())), ..Default::default() }, now()).unwrap();
        assert!(timed.contains("DUE:20261102T140000Z"));
        assert!(apply_patch(&changed, &TaskPatch { due: Some(Some("tomorrow".into())), ..Default::default() }, now()).is_err());
    }

    fn task_of(ics: &str) -> Task {
        task(&Component::parse(ics).unwrap(), "i", "c").unwrap()
    }

    #[test]
    fn creates_tasks() {
        let ics = new_task("uid-1", "Brot, Milch", Some("2026-10-20"), now()).unwrap();
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n"));
        assert!(ics.contains("DUE;VALUE=DATE:20261020\r\n"));
        assert!(ics.contains("STATUS:NEEDS-ACTION\r\n"));
        let task = task_of(&ics);
        assert_eq!((task.title.as_str(), task.due.as_deref(), task.done), ("Brot, Milch", Some("2026-10-20"), false));
        assert!(new_task("u", "x", Some("bald"), now()).is_err());
    }
}
