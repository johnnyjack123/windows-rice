//! Incremental sync of one calendar into the cache: `sync-collection`
//! (RFC 6578) when the server has sync tokens, otherwise CTag + ETag diff.

use crate::cache::Cache;
use crate::client::{DavClient, DavError};
use crate::discovery::Remote;
use crate::webdav::{CALENDAR_QUERY_ETAGS, Multistatus, multiget, sync_collection};
use std::collections::HashMap;
use url::Url;

/// Objects fetched per multiget request.
const BATCH: usize = 50;

fn db(error: rusqlite::Error) -> DavError {
    DavError::Parse(format!("cache: {error}"))
}

/// Brings the cached copy of a calendar up to date. Returns whether anything changed.
pub async fn sync_calendar(client: &DavClient, cache: &Cache, id: &str, remote: &Remote) -> Result<bool, DavError> {
    let (token, ctag) = cache.sync_state(id).map_err(db)?;
    // The listing already says whether anything changed.
    if token.is_some() && token == remote.sync_token {
        return Ok(false);
    }
    if remote.sync_token.is_none() && ctag.is_some() && ctag == remote.ctag {
        return Ok(false);
    }
    if remote.sync_token.is_some() {
        let report = match client.report(&remote.url, 1, &sync_collection(token.as_deref())).await {
            // An expired or unknown token: start over.
            Err(DavError::Status(400 | 403 | 409 | 507)) if token.is_some() => {
                log::info!("calendar: sync token of {id} expired; full resync");
                cache.remove_calendar(id).map_err(db)?;
                client.report(&remote.url, 1, &sync_collection(None)).await?
            }
            other => other?,
        };
        let changed = apply(client, cache, id, &remote.url, &report, true).await?;
        cache.set_sync_state(id, report.sync_token.as_deref().or(remote.sync_token.as_deref()), remote.ctag.as_deref()).map_err(db)?;
        return Ok(changed);
    }
    let listing = client.report(&remote.url, 1, CALENDAR_QUERY_ETAGS).await?;
    let changed = apply(client, cache, id, &remote.url, &listing, false).await?;
    cache.set_sync_state(id, None, remote.ctag.as_deref()).map_err(db)?;
    Ok(changed)
}

/// Fetches new and changed objects and removes deleted ones. In a full
/// listing (`incremental == false`) everything missing was deleted.
async fn apply(client: &DavClient, cache: &Cache, id: &str, base: &Url, report: &Multistatus, incremental: bool) -> Result<bool, DavError> {
    let known: HashMap<String, Option<String>> = cache.etags(id).map_err(db)?;
    let mut fetch = Vec::new();
    let mut removed = Vec::new();
    let mut seen = Vec::new();
    for response in &report.responses {
        if response.href.ends_with('/') {
            continue;
        }
        if response.status == Some(404) {
            removed.push(response.href.clone());
            continue;
        }
        seen.push(response.href.clone());
        let unchanged = response.props.etag.is_some() && known.get(&response.href).is_some_and(|etag| *etag == response.props.etag);
        if !unchanged {
            fetch.push(response.href.clone());
        }
    }
    if !incremental {
        removed.extend(known.keys().filter(|href| !seen.contains(href)).cloned());
    }
    for href in &removed {
        cache.remove(href).map_err(db)?;
    }
    for batch in fetch.chunks(BATCH) {
        let result = client.report(base, 1, &multiget(batch)).await?;
        for object in result.responses {
            if let Some(data) = object.props.calendar_data.as_deref() {
                cache.put(id, &object.href, object.props.etag.as_deref(), data).map_err(db)?;
            }
        }
    }
    Ok(!fetch.is_empty() || !removed.is_empty())
}
