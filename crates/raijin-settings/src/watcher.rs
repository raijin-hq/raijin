use std::path::PathBuf;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Watches a single file for changes and sends the path on each modification.
///
/// Returns a receiver that yields the watched path whenever the file is created,
/// modified, or removed. Uses `notify` with a background thread.
/// The watcher stays alive as long as the returned `WatchHandle` is held.
pub fn watch_file(path: PathBuf) -> (flume::Receiver<PathBuf>, WatchHandle) {
    let (tx, rx) = flume::bounded(16);
    let watched_path = path.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            let Ok(event) = result else { return };
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    let _ = tx.try_send(watched_path.clone());
                }
                _ => {}
            }
        },
        notify::Config::default().with_poll_interval(Duration::from_millis(100)),
    )
    .expect("failed to create file watcher");

    // Watch the parent directory (notify requires existing paths)
    let watch_target = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| path.clone());

    if let Err(err) = watcher.watch(&watch_target, RecursiveMode::NonRecursive) {
        log::warn!("Failed to watch '{}': {err}", watch_target.display());
    }

    (rx, WatchHandle(Some(watcher)))
}

/// Watches a directory for changes and sends changed file paths.
///
/// Returns a receiver that yields paths of files within the directory
/// that were created, modified, or removed.
pub fn watch_dir(path: PathBuf) -> (flume::Receiver<PathBuf>, WatchHandle) {
    let (tx, rx) = flume::bounded(64);

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            let Ok(event) = result else { return };
            match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                    for path in event.paths {
                        let _ = tx.try_send(path);
                    }
                }
                _ => {}
            }
        },
        notify::Config::default().with_poll_interval(Duration::from_millis(100)),
    )
    .expect("failed to create directory watcher");

    if path.is_dir() {
        if let Err(err) = watcher.watch(&path, RecursiveMode::Recursive) {
            log::warn!("Failed to watch directory '{}': {err}", path.display());
        }
    } else {
        log::warn!(
            "Cannot watch non-existent directory: {}",
            path.display()
        );
    }

    (rx, WatchHandle(Some(watcher)))
}

/// Handle that keeps a `notify` watcher alive. Dropping it stops watching.
pub struct WatchHandle(Option<RecommendedWatcher>);

impl Drop for WatchHandle {
    fn drop(&mut self) {
        // Watcher is dropped here, stopping the background thread.
        self.0.take();
    }
}
