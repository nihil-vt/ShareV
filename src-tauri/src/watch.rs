//! Watch-folder tool (ShareX parity): monitors directories for new
//! image files and records them in the history.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::mpsc;

/// Global registry of active folder watchers. Dropping a watcher
/// stops the underlying watch, so removal is just a map removal.
static WATCHERS: Mutex<Option<HashMap<PathBuf, RecommendedWatcher>>> = Mutex::new(None);

/// Image extensions counted as new captures.
const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "bmp", "gif"];

/// Start watching a folder. Returns false when already watched or
/// when the folder cannot be watched.
pub fn watch_add(app: AppHandle, path: PathBuf) -> bool {
    let path = match std::fs::canonicalize(&path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    {
        let mut watchers = WATCHERS.lock().unwrap();
        let map = watchers.get_or_insert_with(HashMap::new);
        if map.contains_key(&path) {
            return false;
        }
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let mut watcher = match notify::RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        notify::Config::default(),
    ) {
        Ok(w) => w,
        Err(err) => {
            log::warn!("could not watch {}: {err}", path.display());
            return false;
        }
    };
    if watcher
        .watch(&path, RecursiveMode::NonRecursive)
        .is_err()
    {
        return false;
    }

    {
        let mut watchers = WATCHERS.lock().unwrap();
        watchers
            .get_or_insert_with(HashMap::new)
            .insert(path.clone(), watcher);
    }

    let known = scan_images(&path);
    // Debounce burst writes: wait for the event stream to settle,
    // then diff the directory contents against the last scan.
    tokio::spawn(async move {
        let mut known = known;
        let mut quiet: Option<std::time::Instant> = None;
        loop {
            match tokio::time::timeout(
                std::time::Duration::from_millis(500),
                rx.recv(),
            )
            .await
            {
                Ok(Some(_)) => quiet = None,
                Ok(None) => break,
                Err(_) => {
                    // Channel idle for the debounce window.
                    let now = std::time::Instant::now();
                    if quiet.is_none() {
                        quiet = Some(now);
                        continue;
                    }
                    quiet = None;
                    let current = scan_images(&path);
                    for fresh in current.difference(&known) {
                        record(app.clone(), path.clone(), fresh.clone());
                    }
                    known = current;
                }
            }
        }
        let mut watchers = WATCHERS.lock().unwrap();
        if let Some(map) = watchers.as_mut() {
            map.remove(&path);
        }
    });

    true
}

/// Stop watching a folder.
pub fn watch_remove(path: &str) -> bool {
    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => return false,
    };
    // Removing the entry drops the watcher, which stops the task
    // via channel close.
    let mut watchers = WATCHERS.lock().unwrap();
    watchers
        .as_mut()
        .and_then(|map| map.remove(&canonical))
        .is_some()
}

/// List watched folders.
pub fn watch_list() -> Vec<String> {
    let watchers = WATCHERS.lock().unwrap();
    watchers
        .as_ref()
        .map(|map| {
            map.keys()
                .map(|p| p.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn scan_images(dir: &PathBuf) -> std::collections::BTreeSet<PathBuf> {
    let mut out = std::collections::BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && IMAGE_EXTS.contains(&ext.to_ascii_lowercase().as_str())
        {
            out.insert(path);
        }
    }
    out
}


fn record(app: AppHandle, dir: PathBuf, path: PathBuf) {
    let Ok((width, height)) = image::image_dimensions(&path) else {
        return;
    };
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let entry = crate::history::HistoryEntry {
        id: crate::id::new_id(),
        path: path.to_string_lossy().into_owned(),
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        kind: "watch".into(),
        width,
        height,
        bytes,
        backend: "watch".into(),
        created_at: chrono::Local::now().to_rfc3339(),
        url: None,
    };
    let max_history = {
        let state = app.state::<crate::AppState>();
        state.config.lock().unwrap().max_history
    };
    if crate::history::append(entry, max_history).is_ok() {
        let _ = app.emit(
            "watch-file",
            serde_json::json!({ "dir": dir.to_string_lossy(), "path": path.to_string_lossy() }),
        );
        log::info!("watch folder captured {}", path.display());
    }
}
#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scans_only_image_extensions() {
        let dir = std::env::temp_dir().join("sharev-watch-scan-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a.png"), b"not a real png").unwrap();
        std::fs::write(dir.join("b.txt"), b"text").unwrap();
        std::fs::write(dir.join("c.JPG"), b"not a real jpg").unwrap();

        let found = scan_images(&dir);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"a.png".to_string()));
        assert!(names.contains(&"c.JPG".to_string()));
        assert!(!names.contains(&"b.txt".to_string()));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn removing_unknown_folder_is_false() {
        assert!(!watch_remove("/definitely/not/a/watched/folder/sharev"));
    }

    #[test]
    fn missing_dir_scan_is_empty() {
        let empty = scan_images(&PathBuf::from("/definitely/missing/sharev"));
        assert!(empty.is_empty());
    }
}
