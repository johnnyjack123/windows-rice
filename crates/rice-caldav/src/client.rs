//! HTTP for CalDAV: PROPFIND/REPORT/GET/PUT/DELETE with Basic auth.
//!
//! TLS comes from Windows (Schannel), so certificates trusted by Windows (also
//! self-signed ones a user installed for a home server) work here too.

use crate::webdav::{Multistatus, parse_multistatus};
use reqwest::header::{CONTENT_TYPE, ETAG, HeaderMap, LOCATION};
use reqwest::{Client, Method, StatusCode};
use std::fmt;
use std::time::Duration;
use url::Url;

const MAX_REDIRECTS: usize = 5;
const XML: &str = "application/xml; charset=utf-8";
const ICS: &str = "text/calendar; charset=utf-8";

/// What went wrong talking to the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DavError {
    /// 401: wrong user name or app password.
    Auth,
    /// The server could not be reached.
    Network(String),
    Timeout,
    /// 412: the object changed on the server since it was read.
    Precondition,
    NotFound,
    Status(u16),
    Parse(String),
}

impl fmt::Display for DavError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auth => write!(f, "authentication failed"),
            Self::Network(e) => write!(f, "network: {e}"),
            Self::Timeout => write!(f, "timeout"),
            Self::Precondition => write!(f, "changed on the server"),
            Self::NotFound => write!(f, "not found"),
            Self::Status(s) => write!(f, "HTTP {s}"),
            Self::Parse(e) => write!(f, "unexpected response: {e}"),
        }
    }
}

impl std::error::Error for DavError {}

impl DavError {
    /// The server is unreachable (rather than refusing the request).
    pub fn is_offline(&self) -> bool {
        matches!(self, Self::Network(_) | Self::Timeout)
    }
}

fn transport(error: reqwest::Error) -> DavError {
    if error.is_timeout() { DavError::Timeout } else { DavError::Network(error.to_string()) }
}

fn check(status: StatusCode) -> Result<(), DavError> {
    match status.as_u16() {
        200..=299 => Ok(()),
        401 => Err(DavError::Auth),
        404 | 410 => Err(DavError::NotFound),
        412 => Err(DavError::Precondition),
        other => Err(DavError::Status(other)),
    }
}

/// Condition of a PUT.
#[derive(Debug, Clone, Copy)]
pub enum Put<'a> {
    /// Only create (`If-None-Match: *`).
    Create,
    /// Only replace this version (`If-Match`).
    Replace(&'a str),
}

/// An authenticated CalDAV client.
pub struct DavClient {
    http: Client,
    user: String,
    password: String,
}

impl DavClient {
    /// A client for one account.
    pub fn new(user: &str, password: &str) -> Result<Self, DavError> {
        let http = Client::builder()
            // Redirects are followed by hand: keep method and body, never leave the server.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("rice/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(transport)?;
        Ok(Self { http, user: user.to_owned(), password: password.to_owned() })
    }

    async fn send(
        &self,
        method: Method,
        url: &Url,
        headers: HeaderMap,
        body: Option<(&'static str, String)>,
    ) -> Result<(Url, reqwest::Response), DavError> {
        let mut url = url.clone();
        for _ in 0..=MAX_REDIRECTS {
            let mut request =
                self.http.request(method.clone(), url.clone()).basic_auth(&self.user, Some(&self.password)).headers(headers.clone());
            if let Some((content_type, body)) = &body {
                request = request.header(CONTENT_TYPE, *content_type).body(body.clone());
            }
            let response = request.send().await.map_err(transport)?;
            if response.status().is_redirection()
                && let Some(location) = response.headers().get(LOCATION).and_then(|l| l.to_str().ok())
            {
                let next = url.join(location).map_err(|e| DavError::Parse(e.to_string()))?;
                if next.origin() != url.origin() {
                    return Err(DavError::Parse(format!("redirect to another server ({})", next.origin().ascii_serialization())));
                }
                url = next;
                continue;
            }
            return Ok((url, response));
        }
        Err(DavError::Parse("too many redirects".into()))
    }

    async fn multistatus(&self, method: &str, url: &Url, depth: u8, body: &str) -> Result<(Url, Multistatus), DavError> {
        let method = Method::from_bytes(method.as_bytes()).map_err(|e| DavError::Parse(e.to_string()))?;
        let mut headers = HeaderMap::new();
        headers.insert("Depth", u32::from(depth).into());
        let (final_url, response) = self.send(method, url, headers, Some((XML, body.to_owned()))).await?;
        check(response.status())?;
        let text = response.text().await.map_err(transport)?;
        Ok((final_url, parse_multistatus(&text).map_err(DavError::Parse)?))
    }

    /// PROPFIND; returns the URL that answered (after redirects) and the result.
    pub async fn propfind(&self, url: &Url, depth: u8, body: &str) -> Result<(Url, Multistatus), DavError> {
        self.multistatus("PROPFIND", url, depth, body).await
    }

    /// REPORT (sync-collection, multiget, calendar-query).
    pub async fn report(&self, url: &Url, depth: u8, body: &str) -> Result<Multistatus, DavError> {
        self.multistatus("REPORT", url, depth, body).await.map(|(_, result)| result)
    }

    /// GET an object: (text, ETag).
    pub async fn get(&self, url: &Url) -> Result<(String, Option<String>), DavError> {
        let (_, response) = self.send(Method::GET, url, HeaderMap::new(), None).await?;
        check(response.status())?;
        let etag = etag(response.headers());
        Ok((response.text().await.map_err(transport)?, etag))
    }

    /// PUT an object; returns the new ETag if the server sent one.
    pub async fn put(&self, url: &Url, ics: &str, condition: Put<'_>) -> Result<Option<String>, DavError> {
        let mut headers = HeaderMap::new();
        match condition {
            Put::Create => {
                headers.insert("If-None-Match", "*".parse().map_err(|_| DavError::Parse("header".into()))?);
            }
            Put::Replace(etag) => {
                headers.insert("If-Match", etag.parse().map_err(|_| DavError::Parse("invalid ETag".into()))?);
            }
        }
        let (_, response) = self.send(Method::PUT, url, headers, Some((ICS, ics.to_owned()))).await?;
        check(response.status())?;
        Ok(etag(response.headers()))
    }

    /// DELETE an object, optionally only if it is still this version.
    pub async fn delete(&self, url: &Url, etag: Option<&str>) -> Result<(), DavError> {
        let mut headers = HeaderMap::new();
        if let Some(etag) = etag {
            headers.insert("If-Match", etag.parse().map_err(|_| DavError::Parse("invalid ETag".into()))?);
        }
        let (_, response) = self.send(Method::DELETE, url, headers, None).await?;
        check(response.status())
    }
}

fn etag(headers: &HeaderMap) -> Option<String> {
    headers.get(ETAG).and_then(|v| v.to_str().ok()).map(str::to_owned)
}
