//! Debounced file watching on top of `notify` (ReadDirectoryChangesW on Windows).

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

/// Keeps a watch alive; dropping it stops watching and ends the worker thread.
pub struct WatchGuard {
    _watcher: RecommendedWatcher,
}

/// Watches `path` and calls `on_change` once per burst of matching events,
/// `debounce` after the last one. Editors write in several steps; this turns
/// them into one reload.
pub fn watch(
    path: &Path,
    recursive: bool,
    debounce: Duration,
    filter: impl Fn(&Event) -> bool + Send + 'static,
    on_change: impl Fn() + Send + 'static,
) -> notify::Result<WatchGuard> {
    let (tx, rx) = mpsc::channel::<()>();
    let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| match result {
        Ok(event) if !matches!(event.kind, EventKind::Access(_)) && filter(&event) => {
            let _ = tx.send(());
        }
        Ok(_) => {}
        Err(error) => log::warn!("watch: {error}"),
    })?;
    let mode = if recursive { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
    watcher.watch(path, mode)?;
    thread::Builder::new()
        .name("rice-watch".into())
        .spawn(move || {
            while rx.recv().is_ok() {
                loop {
                    match rx.recv_timeout(debounce) {
                        Ok(()) => continue,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                on_change();
            }
        })
        .map_err(|e| notify::Error::generic(&e.to_string()))?;
    Ok(WatchGuard { _watcher: watcher })
}

/// Whether an event touches a file with the given name.
pub fn touches_file(event: &Event, name: &str) -> bool {
    event.paths.iter().any(|p| p.file_name().is_some_and(|n| n.eq_ignore_ascii_case(name)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    #[test]
    fn coalesces_a_burst_into_one_callback() {
        let dir = tempfile::tempdir().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&calls);
        let _guard = watch(
            dir.path(),
            false,
            Duration::from_millis(150),
            |event| touches_file(event, "config.json"),
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
            },
        )
        .unwrap();
        for i in 0..5 {
            fs::write(dir.path().join("config.json"), format!("{{\"n\":{i}}}")).unwrap();
        }
        fs::write(dir.path().join("other.txt"), "ignored").unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        while calls.load(Ordering::SeqCst) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(20));
        }
        thread::sleep(Duration::from_millis(300));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
