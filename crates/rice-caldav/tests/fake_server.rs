//! Discovery, sync and task writes against an in-process server that answers
//! like Nextcloud (paths, redirects, auth, sync tokens, ETags).

// Test-only server: failing loudly is the point.
#![allow(clippy::unwrap_used)]

use rice_caldav::cache::Cache;
use rice_caldav::client::{DavClient, DavError, Put};
use rice_caldav::discovery::discover;
use rice_caldav::sync::sync_calendar;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// "joni:secret"
const AUTH: &str = "Basic am9uaTpzZWNyZXQ=";

const EVENT: &str = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Zahnarzt\r\nDTSTART:20261020T080000Z\r\nDTEND:20261020T090000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";

#[derive(Default)]
struct Server {
    /// path → (etag, ics)
    objects: HashMap<String, (String, String)>,
    token: u32,
    /// Requests seen, as "METHOD path".
    log: Vec<String>,
}

fn multistatus(body: &str) -> (u16, Vec<(String, String)>, String) {
    (207, vec![("Content-Type".into(), "application/xml; charset=utf-8".into())], format!(
        r#"<?xml version="1.0"?><d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:cs="http://calendarserver.org/ns/" xmlns:x1="http://apple.com/ns/ical/">{body}</d:multistatus>"#
    ))
}

fn ok_prop(href: &str, props: &str) -> String {
    format!("<d:response><d:href>{href}</d:href><d:propstat><d:prop>{props}</d:prop><d:status>HTTP/1.1 200 OK</d:status></d:propstat></d:response>")
}

impl Server {
    fn handle(&mut self, method: &str, path: &str, headers: &HashMap<String, String>, body: &str) -> (u16, Vec<(String, String)>, String) {
        self.log.push(format!("{method} {path}"));
        if headers.get("authorization").map(String::as_str) != Some(AUTH) {
            return (401, vec![("WWW-Authenticate".into(), "Basic realm=\"Nextcloud\"".into())], String::new());
        }
        let calendar = "/remote.php/dav/calendars/joni/personal/";
        match (method, path) {
            ("PROPFIND", "/.well-known/caldav") => (301, vec![("Location".into(), "/remote.php/dav/".into())], String::new()),
            ("PROPFIND", "/remote.php/dav/") => multistatus(&ok_prop(
                "/remote.php/dav/",
                "<d:current-user-principal><d:href>/remote.php/dav/principals/users/joni/</d:href></d:current-user-principal>",
            )),
            ("PROPFIND", "/remote.php/dav/principals/users/joni/") => multistatus(&ok_prop(
                "/remote.php/dav/principals/users/joni/",
                "<cal:calendar-home-set><d:href>/remote.php/dav/calendars/joni/</d:href></cal:calendar-home-set>",
            )),
            ("PROPFIND", "/remote.php/dav/calendars/joni/") => multistatus(&format!(
                "{}{}",
                ok_prop("/remote.php/dav/calendars/joni/", "<d:resourcetype><d:collection/></d:resourcetype>"),
                ok_prop(
                    calendar,
                    &format!(
                        "<d:resourcetype><d:collection/><cal:calendar/></d:resourcetype><d:displayname>Persönlich</d:displayname>\
<x1:calendar-color>#E9322DFF</x1:calendar-color><cal:supported-calendar-component-set><cal:comp name=\"VEVENT\"/><cal:comp name=\"VTODO\"/></cal:supported-calendar-component-set>\
<d:sync-token>http://sabre.io/ns/sync/{}</d:sync-token>",
                        self.token
                    )
                )
            )),
            ("REPORT", p) if p == calendar && body.contains("sync-collection") => {
                let mut responses: String = self
                    .objects
                    .iter()
                    .map(|(href, (etag, _))| ok_prop(href, &format!("<d:getetag>{etag}</d:getetag>")))
                    .collect();
                responses.push_str(&format!("<d:sync-token>http://sabre.io/ns/sync/{}</d:sync-token>", self.token));
                multistatus(&responses)
            }
            ("REPORT", p) if p == calendar && body.contains("calendar-multiget") => {
                let responses: String = self
                    .objects
                    .iter()
                    .filter(|(href, _)| body.contains(href.as_str()))
                    .map(|(href, (etag, ics))| {
                        ok_prop(href, &format!("<d:getetag>{etag}</d:getetag><cal:calendar-data>{}</cal:calendar-data>", ics.replace('&', "&amp;")))
                    })
                    .collect();
                multistatus(&responses)
            }
            ("PUT", p) => {
                let exists = self.objects.get(p).map(|(etag, _)| etag.clone());
                match (headers.get("if-none-match"), headers.get("if-match"), exists) {
                    (Some(_), _, Some(_)) => (412, vec![], String::new()),
                    (_, Some(wanted), Some(current)) if *wanted != current => (412, vec![], String::new()),
                    (_, Some(_), None) => (412, vec![], String::new()),
                    _ => {
                        self.token += 1;
                        let etag = format!("\"v{}\"", self.token);
                        self.objects.insert(p.to_owned(), (etag.clone(), body.to_owned()));
                        (201, vec![("ETag".into(), etag)], String::new())
                    }
                }
            }
            ("GET", p) => match self.objects.get(p) {
                Some((etag, ics)) => (200, vec![("ETag".into(), etag.clone())], ics.clone()),
                None => (404, vec![], String::new()),
            },
            ("DELETE", p) => {
                self.token += 1;
                if self.objects.remove(p).is_some() { (204, vec![], String::new()) } else { (404, vec![], String::new()) }
            }
            _ => (404, vec![], String::new()),
        }
    }
}

/// Starts the server; returns its base URL and shared state.
async fn start() -> (String, Arc<Mutex<Server>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = Arc::new(Mutex::new(Server::default()));
    state.lock().unwrap().objects.insert("/remote.php/dav/calendars/joni/personal/e1.ics".into(), ("\"e1\"".into(), EVENT.into()));
    let shared = Arc::clone(&state);
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let state = Arc::clone(&shared);
            tokio::spawn(async move {
                let (read, mut write) = stream.into_split();
                let mut reader = BufReader::new(read);
                loop {
                    let mut request_line = String::new();
                    if reader.read_line(&mut request_line).await.unwrap_or(0) == 0 {
                        return;
                    }
                    let mut parts = request_line.split_whitespace();
                    let (method, path) = (parts.next().unwrap_or("").to_owned(), parts.next().unwrap_or("").to_owned());
                    let mut headers = HashMap::new();
                    loop {
                        let mut line = String::new();
                        reader.read_line(&mut line).await.unwrap();
                        let line = line.trim_end();
                        if line.is_empty() {
                            break;
                        }
                        if let Some((k, v)) = line.split_once(':') {
                            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
                        }
                    }
                    let length: usize = headers.get("content-length").and_then(|l| l.parse().ok()).unwrap_or(0);
                    let mut body = vec![0u8; length];
                    reader.read_exact(&mut body).await.unwrap();
                    let body = String::from_utf8_lossy(&body).into_owned();
                    let (status, extra, response) = state.lock().unwrap().handle(&method, &path, &headers, &body);
                    let mut head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\n", response.len());
                    for (k, v) in extra {
                        head.push_str(&format!("{k}: {v}\r\n"));
                    }
                    head.push_str("\r\n");
                    write.write_all(head.as_bytes()).await.unwrap();
                    write.write_all(response.as_bytes()).await.unwrap();
                }
            });
        }
    });
    (format!("http://{address}"), state)
}

#[tokio::test]
async fn discovers_syncs_and_writes_tasks() {
    let (base, server) = start().await;

    // Wrong password: auth_failed, no further paths tried.
    let wrong = DavClient::new("joni", "nope").unwrap();
    assert_eq!(discover(&wrong, &base).await.unwrap_err(), DavError::Auth);

    let client = DavClient::new("joni", "secret").unwrap();
    let found = discover(&client, &base).await.unwrap();
    assert_eq!(found.dav.path(), "/remote.php/dav/");
    assert_eq!(found.home.path(), "/remote.php/dav/calendars/joni/");
    assert_eq!(found.calendars.len(), 1);
    let calendar = &found.calendars[0];
    assert_eq!((calendar.name.as_str(), calendar.color.as_deref(), calendar.events, calendar.tasks), ("Persönlich", Some("#e9322d"), true, true));

    let cache = Cache::in_memory().unwrap();
    assert!(sync_calendar(&client, &cache, "c", calendar).await.unwrap());
    let event = cache.get("/remote.php/dav/calendars/joni/personal/e1.ics").unwrap().unwrap();
    assert!(event.ics.contains("SUMMARY:Zahnarzt"));
    // Same token as in the listing: nothing to do, no request.
    let before = server.lock().unwrap().log.len();
    assert!(!sync_calendar(&client, &cache, "c", calendar).await.unwrap());
    assert_eq!(server.lock().unwrap().log.len(), before);

    // Create, conflict, update, delete.
    let task_url = calendar.url.join("t1.ics").unwrap();
    let ics = rice_caldav::todo::new_task("t1", "Brot", None, chrono::Utc::now()).unwrap();
    let etag = client.put(&task_url, &ics, Put::Create).await.unwrap().unwrap();
    assert_eq!(client.put(&task_url, &ics, Put::Create).await.unwrap_err(), DavError::Precondition);
    assert_eq!(client.put(&task_url, &ics, Put::Replace("\"stale\"")).await.unwrap_err(), DavError::Precondition);
    let (stored, current) = client.get(&task_url).await.unwrap();
    assert_eq!((stored.as_str(), current.as_deref()), (ics.as_str(), Some(etag.as_str())));
    client.delete(&task_url, Some(&etag)).await.unwrap();
    assert_eq!(client.delete(&task_url, None).await.unwrap_err(), DavError::NotFound);

    // The server moved on (new token): a sync picks up the change list again.
    let listing = rice_caldav::discovery::list_calendars(&client, &found.home).await.unwrap();
    assert_ne!(listing[0].sync_token, calendar.sync_token);
    sync_calendar(&client, &cache, "c", &listing[0]).await.unwrap();
    assert!(cache.get("/remote.php/dav/calendars/joni/personal/e1.ics").unwrap().is_some());

    let log = server.lock().unwrap().log.clone();
    assert!(log.contains(&"PROPFIND /.well-known/caldav".to_owned()));
}

#[tokio::test]
async fn reports_unreachable_servers() {
    let client = DavClient::new("joni", "secret").unwrap();
    // Port 9 (discard) on localhost is closed on Windows dev machines.
    let error = discover(&client, "http://127.0.0.1:9").await.unwrap_err();
    assert!(error.is_offline(), "{error:?}");
}
