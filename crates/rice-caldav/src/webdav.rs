//! WebDAV/CalDAV request bodies and `multistatus` parsing (RFC 4918, 4791,
//! 6578). Matching is by namespace, not prefix: servers choose prefixes freely.

use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::{Namespace, ResolveResult};
use rice_core::network::xml_escape;

const DAV: &str = "DAV:";
const CALDAV: &str = "urn:ietf:params:xml:ns:caldav";
const APPLE: &str = "http://apple.com/ns/ical/";
const CALSERVER: &str = "http://calendarserver.org/ns/";

/// PROPFIND for the principal URL.
pub const PROPFIND_PRINCIPAL: &str =
    r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:current-user-principal/></d:prop></d:propfind>"#;

/// PROPFIND for the calendar home.
pub const PROPFIND_HOME: &str = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><c:calendar-home-set/></d:prop></d:propfind>"#;

/// PROPFIND (depth 1) listing calendar collections.
pub const PROPFIND_CALENDARS: &str = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:a="http://apple.com/ns/ical/"><d:prop><d:resourcetype/><d:displayname/><a:calendar-color/><c:supported-calendar-component-set/><cs:getctag/><d:sync-token/></d:prop></d:propfind>"#;

/// REPORT listing the ETags of every object (CTag fallback).
pub const CALENDAR_QUERY_ETAGS: &str = r#"<?xml version="1.0" encoding="utf-8"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/></d:prop><c:filter><c:comp-filter name="VCALENDAR"/></c:filter></c:calendar-query>"#;

/// REPORT sync-collection (RFC 6578); no token means "everything".
pub fn sync_collection(token: Option<&str>) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><d:sync-collection xmlns:d="DAV:"><d:sync-token>{}</d:sync-token><d:sync-level>1</d:sync-level><d:prop><d:getetag/></d:prop></d:sync-collection>"#,
        xml_escape(token.unwrap_or_default())
    )
}

/// REPORT calendar-multiget for the given object paths.
pub fn multiget(hrefs: &[String]) -> String {
    let hrefs: String = hrefs.iter().map(|h| format!("<d:href>{}</d:href>", xml_escape(h))).collect();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><c:calendar-multiget xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/><c:calendar-data/></d:prop>{hrefs}</c:calendar-multiget>"#
    )
}

/// Properties of one resource (only those with status 200).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Props {
    pub displayname: Option<String>,
    pub color: Option<String>,
    /// VEVENT, VTODO …; `None` if the server did not say.
    pub components: Option<Vec<String>>,
    pub is_calendar: bool,
    pub ctag: Option<String>,
    pub sync_token: Option<String>,
    pub etag: Option<String>,
    pub calendar_data: Option<String>,
    pub principal: Option<String>,
    pub home: Option<String>,
}

/// One `<response>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Response {
    pub href: String,
    /// Response-level status (e.g. 404 for deletions in sync-collection).
    pub status: Option<u16>,
    pub props: Props,
}

/// A parsed `<multistatus>`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Multistatus {
    pub responses: Vec<Response>,
    /// New token of a sync-collection report.
    pub sync_token: Option<String>,
}

/// "HTTP/1.1 404 Not Found" → 404
fn status_code(line: &str) -> Option<u16> {
    line.split_whitespace().nth(1)?.parse().ok()
}

fn ns(result: &ResolveResult<'_>) -> &'static str {
    match result {
        ResolveResult::Bound(Namespace(n)) if *n == DAV => DAV,
        ResolveResult::Bound(Namespace(n)) if *n == CALDAV => CALDAV,
        ResolveResult::Bound(Namespace(n)) if *n == APPLE => APPLE,
        ResolveResult::Bound(Namespace(n)) if *n == CALSERVER => CALSERVER,
        _ => "",
    }
}

fn entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

/// Parses a multistatus body.
pub fn parse_multistatus(xml: &str) -> Result<Multistatus, String> {
    let mut reader = NsReader::from_str(xml);
    let mut out = Multistatus::default();
    let mut stack: Vec<(&'static str, String)> = Vec::new();
    let mut text = String::new();
    let mut response: Option<Response> = None;
    let mut propstat: Option<(Props, Option<u16>)> = None;
    let inside = |stack: &[(&'static str, String)], namespace: &str, name: &str| stack.iter().any(|(n, l)| *n == namespace && l == name);
    loop {
        let (resolved, event) = reader.read_resolved_event().map_err(|e| e.to_string())?;
        let namespace = ns(&resolved);
        match event {
            Event::Start(start) => {
                let local = start.local_name().as_ref().to_owned();
                text.clear();
                match (namespace, local.as_str()) {
                    (DAV, "response") => response = Some(Response::default()),
                    (DAV, "propstat") => propstat = Some((Props::default(), None)),
                    (CALDAV, "calendar") if inside(&stack, DAV, "resourcetype") => {
                        if let Some((props, _)) = propstat.as_mut() {
                            props.is_calendar = true;
                        }
                    }
                    (CALDAV, "supported-calendar-component-set") => {
                        if let Some((props, _)) = propstat.as_mut() {
                            props.components.get_or_insert_with(Vec::new);
                        }
                    }
                    (CALDAV, "comp") => component(&start, propstat.as_mut()),
                    _ => {}
                }
                stack.push((namespace, local));
            }
            Event::Empty(start) => {
                let local = start.local_name().as_ref().to_owned();
                match (namespace, local.as_str()) {
                    (CALDAV, "calendar") if inside(&stack, DAV, "resourcetype") => {
                        if let Some((props, _)) = propstat.as_mut() {
                            props.is_calendar = true;
                        }
                    }
                    (CALDAV, "supported-calendar-component-set") => {
                        if let Some((props, _)) = propstat.as_mut() {
                            props.components.get_or_insert_with(Vec::new);
                        }
                    }
                    (CALDAV, "comp") => component(&start, propstat.as_mut()),
                    _ => {}
                }
            }
            Event::Text(t) => text.push_str(&t.xml10_content()),
            Event::CData(c) => text.push_str(&c.xml10_content()),
            Event::GeneralRef(r) => {
                let resolved = if r.is_char_ref() { r.resolve_char_ref().ok().flatten() } else { entity(&r.xml10_content()) };
                if let Some(c) = resolved {
                    text.push(c);
                }
            }
            Event::End(_) => {
                let Some((end_ns, local)) = stack.pop() else { continue };
                let value = std::mem::take(&mut text);
                let trimmed = value.trim().to_owned();
                let parent = stack.last().map(|(n, l)| (*n, l.as_str()));
                match (end_ns, local.as_str()) {
                    (DAV, "response") => {
                        if let Some(done) = response.take() {
                            out.responses.push(done);
                        }
                    }
                    (DAV, "propstat") => {
                        if let (Some((props, status)), Some(r)) = (propstat.take(), response.as_mut())
                            && status.is_none_or(|s| (200..300).contains(&s))
                        {
                            merge(&mut r.props, props);
                        }
                    }
                    (DAV, "status") => match (parent, propstat.as_mut(), response.as_mut()) {
                        (Some((DAV, "propstat")), Some((_, status)), _) => *status = status_code(&trimmed),
                        (Some((DAV, "response")), _, Some(r)) => r.status = status_code(&trimmed),
                        _ => {}
                    },
                    (DAV, "href") => match (parent, propstat.as_mut(), response.as_mut()) {
                        (Some((DAV, "response")), _, Some(r)) => r.href = trimmed,
                        (Some((DAV, "current-user-principal")), Some((props, _)), _) => props.principal = Some(trimmed),
                        (Some((CALDAV, "calendar-home-set")), Some((props, _)), _) => props.home = Some(trimmed),
                        _ => {}
                    },
                    (DAV, "sync-token") if parent == Some((DAV, "multistatus")) => out.sync_token = Some(trimmed),
                    (namespace, name) => {
                        if let Some((props, _)) = propstat.as_mut() {
                            match (namespace, name) {
                                (DAV, "displayname") => props.displayname = Some(trimmed),
                                (APPLE, "calendar-color") => props.color = Some(trimmed),
                                (CALSERVER, "getctag") => props.ctag = Some(trimmed),
                                (DAV, "sync-token") => props.sync_token = Some(trimmed),
                                (DAV, "getetag") => props.etag = Some(trimmed),
                                // Keep calendar data verbatim (line breaks matter).
                                (CALDAV, "calendar-data") => props.calendar_data = Some(value),
                                _ => {}
                            }
                        }
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

fn component(start: &quick_xml::events::BytesStart<'_>, propstat: Option<&mut (Props, Option<u16>)>) {
    let Some((props, _)) = propstat else { return };
    if let Ok(Some(name)) = start.try_get_attribute("name")
        && let Ok(value) = name.normalized_value(quick_xml::XmlVersion::Implicit1_0)
    {
        props.components.get_or_insert_with(Vec::new).push(value.to_ascii_uppercase());
    }
}

fn merge(into: &mut Props, from: Props) {
    into.displayname = from.displayname.or(into.displayname.take());
    into.color = from.color.or(into.color.take());
    into.components = from.components.or(into.components.take());
    into.is_calendar |= from.is_calendar;
    into.ctag = from.ctag.or(into.ctag.take());
    into.sync_token = from.sync_token.or(into.sync_token.take());
    into.etag = from.etag.or(into.etag.take());
    into.calendar_data = from.calendar_data.or(into.calendar_data.take());
    into.principal = from.principal.or(into.principal.take());
    into.home = from.home.or(into.home.take());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_principal_and_home() {
        let principal = r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:s="http://sabredav.org/ns">
<d:response><d:href>/remote.php/dav/</d:href><d:propstat><d:prop><d:current-user-principal><d:href>/remote.php/dav/principals/users/joni/</d:href></d:current-user-principal></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#;
        let parsed = parse_multistatus(principal).unwrap();
        assert_eq!(parsed.responses[0].href, "/remote.php/dav/");
        assert_eq!(parsed.responses[0].props.principal.as_deref(), Some("/remote.php/dav/principals/users/joni/"));
        let home = r#"<d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav"><d:response><d:href>/p/</d:href>
<d:propstat><d:prop><cal:calendar-home-set><d:href>/remote.php/dav/calendars/joni/</d:href></cal:calendar-home-set></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>"#;
        assert_eq!(parse_multistatus(home).unwrap().responses[0].props.home.as_deref(), Some("/remote.php/dav/calendars/joni/"));
    }

    #[test]
    fn parses_calendar_listings_with_any_prefixes() {
        let xml = r#"<d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:x1="http://apple.com/ns/ical/">
<d:response><d:href>/remote.php/dav/calendars/joni/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/></d:resourcetype></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
<d:response><d:href>/remote.php/dav/calendars/joni/personal/</d:href>
<d:propstat><d:prop><d:resourcetype><d:collection/><cal:calendar/></d:resourcetype><d:displayname>Persönlich &amp; Arbeit</d:displayname>
<x1:calendar-color>#0082C9FF</x1:calendar-color><cal:supported-calendar-component-set><cal:comp name="VEVENT"/></cal:supported-calendar-component-set>
<cs:getctag>"ctag-1"</cs:getctag><d:sync-token>http://sabre.io/ns/sync/42</d:sync-token></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat>
<d:propstat><d:prop><d:displayname/></d:prop><d:status>HTTP/1.1 404 Not Found</d:status></d:propstat></d:response>
<d:response><d:href>/remote.php/dav/calendars/joni/tasks/</d:href><d:propstat><d:prop><d:resourcetype><d:collection/><cal:calendar/></d:resourcetype>
<d:displayname>Aufgaben</d:displayname><cal:supported-calendar-component-set><cal:comp name="VTODO"/></cal:supported-calendar-component-set></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
</d:multistatus>"#;
        let parsed = parse_multistatus(xml).unwrap();
        assert_eq!(parsed.responses.len(), 3);
        assert!(!parsed.responses[0].props.is_calendar);
        let personal = &parsed.responses[1].props;
        assert!(personal.is_calendar);
        assert_eq!(personal.displayname.as_deref(), Some("Persönlich & Arbeit"));
        assert_eq!(personal.color.as_deref(), Some("#0082C9FF"));
        assert_eq!(personal.components.as_deref(), Some(&["VEVENT".to_owned()][..]));
        assert_eq!(personal.ctag.as_deref(), Some("\"ctag-1\""));
        assert_eq!(personal.sync_token.as_deref(), Some("http://sabre.io/ns/sync/42"));
        assert_eq!(parsed.responses[2].props.components.as_deref(), Some(&["VTODO".to_owned()][..]));
    }

    #[test]
    fn parses_sync_and_multiget_reports() {
        let sync = r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/c/a.ics</d:href><d:propstat><d:prop><d:getetag>"e1"</d:getetag></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>
<d:response><d:href>/c/gone.ics</d:href><d:status>HTTP/1.1 404 Not Found</d:status></d:response><d:sync-token>http://sabre.io/ns/sync/43</d:sync-token></d:multistatus>"#;
        let parsed = parse_multistatus(sync).unwrap();
        assert_eq!(parsed.sync_token.as_deref(), Some("http://sabre.io/ns/sync/43"));
        assert_eq!(parsed.responses[0].props.etag.as_deref(), Some("\"e1\""));
        assert_eq!(parsed.responses[1].status, Some(404));

        let multiget = "<d:multistatus xmlns:d=\"DAV:\" xmlns:cal=\"urn:ietf:params:xml:ns:caldav\"><d:response><d:href>/c/a.ics</d:href><d:propstat><d:prop>\
<d:getetag>\"e1\"</d:getetag><cal:calendar-data>BEGIN:VCALENDAR\r\nSUMMARY:Tom &amp; Jerry\r\nEND:VCALENDAR\r\n</cal:calendar-data></d:prop>\
<d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>";
        let data = parse_multistatus(multiget).unwrap().responses[0].props.calendar_data.clone().unwrap();
        assert!(data.contains("SUMMARY:Tom & Jerry"));
        let cdata = "<d:multistatus xmlns:d=\"DAV:\" xmlns:C=\"urn:ietf:params:xml:ns:caldav\"><d:response><d:href>/x</d:href><d:propstat><d:prop>\
<C:calendar-data><![CDATA[BEGIN:VCALENDAR\nEND:VCALENDAR\n]]></C:calendar-data></d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response></d:multistatus>";
        assert_eq!(parse_multistatus(cdata).unwrap().responses[0].props.calendar_data.as_deref(), Some("BEGIN:VCALENDAR\nEND:VCALENDAR\n"));
    }

    #[test]
    fn escapes_request_values() {
        assert!(sync_collection(Some("a&b")).contains("<d:sync-token>a&amp;b</d:sync-token>"));
        assert!(sync_collection(None).contains("<d:sync-token></d:sync-token>"));
        assert!(multiget(&["/c/a&b.ics".into()]).contains("<d:href>/c/a&amp;b.ics</d:href>"));
    }

    #[test]
    fn rejects_malformed_xml() {
        assert!(parse_multistatus("<d:multistatus xmlns:d=\"DAV:\"><d:response></d:multistatus>").is_err());
        assert_eq!(parse_multistatus("").unwrap(), Multistatus::default());
    }
}
