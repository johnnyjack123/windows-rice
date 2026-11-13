//! A small iCalendar (RFC 5545) reader and writer that keeps everything it
//! does not understand. Other clients (DAVx⁵, Tasks.org, Thunderbird) store
//! their own properties; editing a task must not drop them.

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeDelta, Utc};

/// One content line: `NAME;PARAM=value:VALUE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Upper case.
    pub name: String,
    /// Parameter names upper case, values without quotes.
    pub params: Vec<(String, String)>,
    /// Raw value, still escaped.
    pub value: String,
}

impl Property {
    /// A property without parameters.
    pub fn new(name: &str, value: impl Into<String>) -> Self {
        Self { name: name.to_ascii_uppercase(), params: Vec::new(), value: value.into() }
    }

    /// A TEXT property (value gets escaped).
    pub fn text(name: &str, text: &str) -> Self {
        Self::new(name, escape_text(text))
    }

    /// Adds a parameter.
    pub fn with_param(mut self, name: &str, value: &str) -> Self {
        self.params.push((name.to_ascii_uppercase(), value.to_owned()));
        self
    }

    /// Parses an unfolded content line.
    pub fn parse(line: &str) -> Option<Self> {
        let mut in_quotes = false;
        let mut split = None;
        for (i, c) in line.char_indices() {
            match c {
                '"' => in_quotes = !in_quotes,
                ':' if !in_quotes => {
                    split = Some(i);
                    break;
                }
                _ => {}
            }
        }
        let split = split?;
        let (head, value) = (&line[..split], &line[split + 1..]);
        let mut parts = split_unquoted(head, ';').into_iter();
        let name = parts.next()?.trim().to_ascii_uppercase();
        if name.is_empty() {
            return None;
        }
        let params = parts
            .filter_map(|p| {
                let (key, value) = p.split_once('=')?;
                Some((key.trim().to_ascii_uppercase(), value.trim_matches('"').to_owned()))
            })
            .collect();
        Some(Self { name, params, value: value.to_owned() })
    }

    /// A parameter's value.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    /// The value as TEXT.
    pub fn text_value(&self) -> String {
        unescape_text(&self.value)
    }

    /// The content line (unfolded).
    pub fn to_line(&self) -> String {
        let mut line = self.name.clone();
        for (key, value) in &self.params {
            line.push(';');
            line.push_str(key);
            line.push('=');
            if value.contains([':', ';', ',']) {
                line.push('"');
                line.push_str(value);
                line.push('"');
            } else {
                line.push_str(value);
            }
        }
        line.push(':');
        line.push_str(&self.value);
        line
    }
}

fn split_unquoted(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    for (i, c) in text.char_indices() {
        if c == '"' {
            in_quotes = !in_quotes;
        } else if c == separator && !in_quotes {
            parts.push(&text[start..i]);
            start = i + c.len_utf8();
        }
    }
    parts.push(&text[start..]);
    parts
}

/// A component (`BEGIN:X` … `END:X`) with its properties and children in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Component {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: Vec<Component>,
}

impl Component {
    /// A new, empty component.
    pub fn new(name: &str) -> Self {
        Self { name: name.to_ascii_uppercase(), ..Self::default() }
    }

    /// Parses an iCalendar object; returns its outermost component (VCALENDAR).
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut stack: Vec<Component> = Vec::new();
        let mut root = None;
        for line in unfold(text) {
            if line.trim().is_empty() {
                continue;
            }
            let Some(property) = Property::parse(&line) else { continue };
            match property.name.as_str() {
                "BEGIN" => stack.push(Component::new(property.value.trim())),
                "END" => {
                    let done = stack.pop().ok_or("END without BEGIN")?;
                    if !done.name.eq_ignore_ascii_case(property.value.trim()) {
                        return Err(format!("END:{} closes {}", property.value.trim(), done.name));
                    }
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(done),
                        None => root = Some(done),
                    }
                }
                _ => match stack.last_mut() {
                    Some(current) => current.properties.push(property),
                    None => return Err("content outside a component".into()),
                },
            }
            if root.is_some() {
                break;
            }
        }
        root.ok_or_else(|| "no complete component".into())
    }

    /// First property with this name.
    pub fn property(&self, name: &str) -> Option<&Property> {
        self.properties.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// All properties with this name.
    pub fn properties_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Property> {
        self.properties.iter().filter(move |p| p.name.eq_ignore_ascii_case(name))
    }

    /// TEXT value of a property.
    pub fn text(&self, name: &str) -> Option<String> {
        self.property(name).map(Property::text_value)
    }

    /// Replaces the property of that name (all occurrences) or appends it.
    pub fn set(&mut self, property: Property) {
        match self.properties.iter().position(|p| p.name == property.name) {
            Some(index) => {
                self.properties[index] = property.clone();
                let name = property.name;
                let mut seen = false;
                self.properties.retain(|p| {
                    if p.name != name {
                        return true;
                    }
                    let keep = !seen;
                    seen = true;
                    keep
                });
            }
            None => self.properties.push(property),
        }
    }

    /// Removes every property of that name.
    pub fn remove(&mut self, name: &str) {
        self.properties.retain(|p| !p.name.eq_ignore_ascii_case(name));
    }

    /// Child components with this name.
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Component> {
        self.children.iter().filter(move |c| c.name.eq_ignore_ascii_case(name))
    }

    /// First mutable child with this name.
    pub fn child_mut(&mut self, name: &str) -> Option<&mut Component> {
        self.children.iter_mut().find(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Serialises with CRLF line ends and lines folded at 75 octets.
    pub fn to_ics(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        out.push_str(&fold(&format!("BEGIN:{}", self.name)));
        for property in &self.properties {
            out.push_str(&fold(&property.to_line()));
        }
        for child in &self.children {
            child.write(out);
        }
        out.push_str(&fold(&format!("END:{}", self.name)));
    }
}

/// Joins folded lines (a line starting with space or tab continues the previous one).
pub fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        match (raw.strip_prefix([' ', '\t']), lines.last_mut()) {
            (Some(rest), Some(last)) => last.push_str(rest),
            _ => lines.push(raw.to_owned()),
        }
    }
    lines
}

/// Folds a content line at 75 octets without splitting a UTF-8 character;
/// ends with CRLF.
pub fn fold(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 8);
    let mut width = 0;
    for c in line.chars() {
        let len = c.len_utf8();
        if width + len > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += len;
    }
    out.push_str("\r\n");
    out
}

/// Escapes TEXT values (`\`, `;`, `,`, newlines).
pub fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Reverses [`escape_text`].
pub fn unescape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// A DATE or DATE-TIME value with its time zone reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Time {
    Date(NaiveDate),
    Utc(NaiveDateTime),
    /// No zone: the local time wherever the user is ("floating").
    Floating(NaiveDateTime),
    /// Local time in the zone named by TZID.
    Zoned(NaiveDateTime, String),
}

impl Time {
    /// Whether this is a date without time (all-day).
    pub fn is_date(&self) -> bool {
        matches!(self, Self::Date(_))
    }
}

/// Parses one DATE or DATE-TIME value.
pub fn parse_time_value(value: &str, date_param: bool, tzid: Option<&str>) -> Option<Time> {
    let value = value.trim();
    if date_param || value.len() == 8 {
        return NaiveDate::parse_from_str(value, "%Y%m%d").ok().map(Time::Date);
    }
    let (local, utc) = match value.strip_suffix(['Z', 'z']) {
        Some(stripped) => (stripped, true),
        None => (value, false),
    };
    let time = NaiveDateTime::parse_from_str(local, "%Y%m%dT%H%M%S").ok()?;
    Some(match (utc, tzid) {
        (true, _) => Time::Utc(time),
        (false, Some(tzid)) if !tzid.is_empty() => Time::Zoned(time, tzid.to_owned()),
        (false, _) => Time::Floating(time),
    })
}

/// The (first) time of a DTSTART/DTEND/DUE/… property.
pub fn parse_time(property: &Property) -> Option<Time> {
    let first = property.value.split(',').next()?;
    parse_time_value(first, is_date_param(property), property.param("TZID"))
}

/// Every time of a list property (EXDATE, RDATE).
pub fn parse_times(property: &Property) -> Vec<Time> {
    let date = is_date_param(property);
    property.value.split(',').filter_map(|v| parse_time_value(v, date, property.param("TZID"))).collect()
}

fn is_date_param(property: &Property) -> bool {
    property.param("VALUE").is_some_and(|v| v.eq_ignore_ascii_case("DATE"))
}

/// `YYYYMMDD`
pub fn format_date(date: NaiveDate) -> String {
    date.format("%Y%m%d").to_string()
}

/// `YYYYMMDDTHHMMSSZ`
pub fn format_utc(time: DateTime<Utc>) -> String {
    time.format("%Y%m%dT%H%M%SZ").to_string()
}

/// `YYYYMMDDTHHMMSS` (no zone)
pub fn format_local(time: NaiveDateTime) -> String {
    time.format("%Y%m%dT%H%M%S").to_string()
}

/// Parses an iCalendar DURATION (`P1D`, `PT1H30M`, `-PT15M`, `P2W`).
pub fn parse_duration(value: &str) -> Option<TimeDelta> {
    let value = value.trim();
    let (negative, rest) = match value.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let rest = rest.strip_prefix(['P', 'p'])?;
    let mut total = TimeDelta::zero();
    let mut number = String::new();
    let mut in_time = false;
    for c in rest.chars() {
        match c.to_ascii_uppercase() {
            'T' => in_time = true,
            d if d.is_ascii_digit() => number.push(d),
            unit => {
                let n: i64 = number.parse().ok()?;
                number.clear();
                total += match (unit, in_time) {
                    ('W', false) => TimeDelta::try_weeks(n)?,
                    ('D', false) => TimeDelta::try_days(n)?,
                    ('H', true) => TimeDelta::try_hours(n)?,
                    ('M', true) => TimeDelta::try_minutes(n)?,
                    ('S', true) => TimeDelta::try_seconds(n)?,
                    _ => return None,
                };
            }
        }
    }
    if !number.is_empty() {
        return None;
    }
    Some(if negative { -total } else { total })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Example//EN\r\nBEGIN:VTODO\r\nUID:abc-1\r\nSUMMARY:Milch\\, Brot\r\n  und Eier\r\nX-APPLE-SORT-ORDER:12\r\nDUE;VALUE=DATE:20261020\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";

    #[test]
    fn parses_properties_with_quoted_parameters() {
        let p = Property::parse("ATTENDEE;CN=\"Doe; John\";ROLE=REQ-PARTICIPANT:mailto:j@example.org").unwrap();
        assert_eq!(p.name, "ATTENDEE");
        assert_eq!(p.param("cn"), Some("Doe; John"));
        assert_eq!(p.value, "mailto:j@example.org");
        assert_eq!(Property::parse(&p.to_line()).unwrap(), p);
        assert!(Property::parse("no colon here").is_none());
    }

    #[test]
    fn round_trips_unknown_content() {
        let calendar = Component::parse(SAMPLE).unwrap();
        let todo = calendar.children_named("VTODO").next().unwrap();
        assert_eq!(todo.text("SUMMARY").as_deref(), Some("Milch, Brot und Eier"));
        assert_eq!(todo.property("X-APPLE-SORT-ORDER").unwrap().value, "12");
        let again = Component::parse(&calendar.to_ics()).unwrap();
        assert_eq!(again, calendar);
    }

    #[test]
    fn edits_properties() {
        let mut calendar = Component::parse(SAMPLE).unwrap();
        let todo = calendar.child_mut("VTODO").unwrap();
        todo.set(Property::text("SUMMARY", "Neu; besser"));
        todo.set(Property::new("STATUS", "COMPLETED"));
        todo.remove("DUE");
        assert_eq!(todo.text("SUMMARY").as_deref(), Some("Neu; besser"));
        assert_eq!(todo.property("SUMMARY").unwrap().value, "Neu\\; besser");
        assert!(todo.property("DUE").is_none());
        assert_eq!(todo.properties_named("STATUS").count(), 1);
    }

    #[test]
    fn rejects_broken_structure() {
        assert!(Component::parse("BEGIN:VCALENDAR\r\nBEGIN:VTODO\r\nEND:VEVENT\r\n").is_err());
        assert!(Component::parse("SUMMARY:x").is_err());
        assert!(Component::parse("").is_err());
    }

    #[test]
    fn folds_long_lines_on_character_boundaries() {
        let line = format!("SUMMARY:{}", "ü".repeat(60));
        let folded = fold(&line);
        assert!(folded.split("\r\n").all(|l| l.len() <= 75));
        assert_eq!(unfold(&folded)[0], line);
    }

    #[test]
    fn escapes_text() {
        let text = "a,b;c\\d\nnew line";
        assert_eq!(escape_text(text), "a\\,b\\;c\\\\d\\nnew line");
        assert_eq!(unescape_text(&escape_text(text)), text);
    }

    #[test]
    fn parses_times() {
        let date = Property::parse("DTSTART;VALUE=DATE:20261020").unwrap();
        assert_eq!(parse_time(&date), NaiveDate::from_ymd_opt(2026, 10, 20).map(Time::Date));
        let zoned = Property::parse("DTSTART;TZID=Europe/Berlin:20261020T100000").unwrap();
        assert!(matches!(parse_time(&zoned), Some(Time::Zoned(_, z)) if z == "Europe/Berlin"));
        let utc = Property::parse("DTSTART:20261020T080000Z").unwrap();
        assert!(matches!(parse_time(&utc), Some(Time::Utc(_))));
        let floating = Property::parse("DTSTART:20261020T080000").unwrap();
        assert!(matches!(parse_time(&floating), Some(Time::Floating(_))));
        let list = Property::parse("EXDATE:20261020T080000Z,20261027T080000Z").unwrap();
        assert_eq!(parse_times(&list).len(), 2);
        assert!(parse_time(&Property::parse("DTSTART:yesterday").unwrap()).is_none());
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration("PT1H30M"), TimeDelta::try_minutes(90));
        assert_eq!(parse_duration("P1D"), TimeDelta::try_days(1));
        assert_eq!(parse_duration("P2W"), TimeDelta::try_weeks(2));
        assert_eq!(parse_duration("-PT15M"), TimeDelta::try_minutes(-15));
        assert_eq!(parse_duration("P1DT12H"), TimeDelta::try_hours(36));
        assert_eq!(parse_duration("PT"), Some(TimeDelta::zero()));
        assert_eq!(parse_duration("1H"), None);
        assert_eq!(parse_duration("PT5"), None);
    }
}
