//! Finding the calendars of an account: DAV root → principal → calendar
//! home → collections (RFC 4791 §6, RFC 6764 well-known URL).

use crate::client::{DavClient, DavError};
use crate::webdav::{PROPFIND_CALENDARS, PROPFIND_HOME, PROPFIND_PRINCIPAL};
use url::Url;

/// A calendar collection on the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub url: Url,
    pub name: String,
    /// "#rrggbb"
    pub color: Option<String>,
    pub events: bool,
    pub tasks: bool,
    pub ctag: Option<String>,
    pub sync_token: Option<String>,
}

/// Result of a discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovered {
    /// The DAV root that answered.
    pub dav: Url,
    pub home: Url,
    pub calendars: Vec<Remote>,
}

/// Accepts "cloud.example.org", "https://cloud.example.org/" or a full DAV URL.
pub fn normalise_url(input: &str) -> Result<Url, DavError> {
    let input = input.trim();
    let with_scheme = if input.contains("://") { input.to_owned() } else { format!("https://{input}") };
    let mut url = Url::parse(&with_scheme).map_err(|e| DavError::Parse(format!("invalid address: {e}")))?;
    if !matches!(url.scheme(), "https" | "http") || url.host().is_none() {
        return Err(DavError::Parse("the address must be http(s)://server".into()));
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

/// Where to look for the DAV root, most specific first.
pub fn candidates(base: &Url) -> Vec<Url> {
    let mut list = Vec::new();
    if base.path().contains("/dav") {
        list.push(base.clone());
    }
    for path in ["/.well-known/caldav", "remote.php/dav/"] {
        if let Ok(url) = base.join(path)
            && !list.contains(&url)
        {
            list.push(url);
        }
    }
    if !list.contains(base) {
        list.push(base.clone());
    }
    list
}

/// "#0082C9FF" → "#0082c9"
pub fn color(raw: &str) -> Option<String> {
    let six = raw.trim().strip_prefix('#')?.get(..6)?;
    six.chars().all(|c| c.is_ascii_hexdigit()).then(|| format!("#{}", six.to_ascii_lowercase()))
}

/// Discovers principal, calendar home and calendars.
pub async fn discover(client: &DavClient, input: &str) -> Result<Discovered, DavError> {
    let base = normalise_url(input)?;
    let mut found = None;
    let mut last_error = DavError::NotFound;
    for candidate in candidates(&base) {
        match client.propfind(&candidate, 0, PROPFIND_PRINCIPAL).await {
            Ok((answered, result)) => {
                if let Some(principal) = result.responses.iter().find_map(|r| r.props.principal.clone()) {
                    found = Some((answered.clone(), answered.join(&principal).map_err(|e| DavError::Parse(e.to_string()))?));
                    break;
                }
            }
            // Wrong credentials or an unreachable server will not get better on another path.
            Err(error @ (DavError::Auth | DavError::Network(_) | DavError::Timeout)) => return Err(error),
            Err(error) => last_error = error,
        }
    }
    let (dav, principal) = found.ok_or(last_error)?;
    let (_, result) = client.propfind(&principal, 0, PROPFIND_HOME).await?;
    let home = result
        .responses
        .iter()
        .find_map(|r| r.props.home.clone())
        .ok_or_else(|| DavError::Parse("the server has no calendar home".into()))?;
    let home = principal.join(&home).map_err(|e| DavError::Parse(e.to_string()))?;
    let calendars = list_calendars(client, &home).await?;
    Ok(Discovered { dav, home, calendars })
}

/// The calendar collections below the home.
pub async fn list_calendars(client: &DavClient, home: &Url) -> Result<Vec<Remote>, DavError> {
    let (_, result) = client.propfind(home, 1, PROPFIND_CALENDARS).await?;
    Ok(result
        .responses
        .into_iter()
        .filter(|r| r.props.is_calendar)
        .filter_map(|r| {
            let url = home.join(&r.href).ok()?;
            if url == *home {
                return None;
            }
            let components = r.props.components.clone();
            let has = |name: &str| components.as_ref().is_none_or(|c| c.iter().any(|x| x == name));
            let fallback = url.path_segments().and_then(|mut s| s.rfind(|p| !p.is_empty()).map(str::to_owned)).unwrap_or_default();
            Some(Remote {
                name: r.props.displayname.filter(|n| !n.trim().is_empty()).unwrap_or(fallback),
                color: r.props.color.as_deref().and_then(color),
                events: has("VEVENT"),
                tasks: has("VTODO"),
                ctag: r.props.ctag,
                sync_token: r.props.sync_token,
                url,
            })
        })
        .filter(|r| r.events || r.tasks)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_addresses() {
        assert_eq!(normalise_url("cloud.example.org").unwrap().as_str(), "https://cloud.example.org/");
        assert_eq!(normalise_url(" https://example.org/nextcloud ").unwrap().as_str(), "https://example.org/nextcloud/");
        assert_eq!(normalise_url("http://nas:8080/remote.php/dav").unwrap().as_str(), "http://nas:8080/remote.php/dav/");
        assert!(normalise_url("ftp://example.org").is_err());
        assert!(normalise_url("https://").is_err());
    }

    #[test]
    fn tries_the_given_dav_path_first() {
        let paths = |u: &str| candidates(&normalise_url(u).unwrap()).into_iter().map(|u| u.to_string()).collect::<Vec<_>>();
        assert_eq!(
            paths("https://example.org/nextcloud"),
            [
                "https://example.org/.well-known/caldav",
                "https://example.org/nextcloud/remote.php/dav/",
                "https://example.org/nextcloud/",
            ]
        );
        assert_eq!(paths("https://example.org/remote.php/dav")[0], "https://example.org/remote.php/dav/");
    }

    #[test]
    fn normalises_colours() {
        assert_eq!(color("#0082C9FF").as_deref(), Some("#0082c9"));
        assert_eq!(color("#FF0000").as_deref(), Some("#ff0000"));
        assert_eq!(color("red"), None);
        assert_eq!(color("#12"), None);
        assert_eq!(color("#ääää"), None);
    }
}
