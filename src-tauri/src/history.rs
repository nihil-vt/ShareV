use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths;

/// Serializes all read-modify-write cycles so concurrent callers (e.g. the
/// auto-capture loop and the UI) can't lose updates.
fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub path: String,
    pub file_name: String,
    /// region | fullscreen | window
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    /// Capture backend that produced the image.
    pub backend: String,
    /// RFC3339 timestamp.
    pub created_at: String,
    /// URL returned by the last successful upload of this image.
    #[serde(default)]
    pub url: Option<String>,
}

fn read_all() -> Vec<HistoryEntry> {
    let path = paths::history_file();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    match serde_json::from_str(&text) {
        Ok(entries) => entries,
        Err(err) => {
            // Don't silently drop everything: surface the problem, then start
            // fresh rather than repeatedly failing on the same broken file.
            log::warn!("could not parse history file {}: {err}", path.display());
            Vec::new()
        }
    }
}

fn write_all(entries: &[HistoryEntry]) -> Result<()> {
    let path = paths::history_file();
    paths::ensure_parent(&path)?;
    let text = serde_json::to_string_pretty(entries)?;
    // Write to a sibling temp file and rename so a crash mid-write can never
    // leave a truncated history file behind.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Newest first.
pub fn list(limit: Option<usize>) -> Result<Vec<HistoryEntry>> {
    let all = read_all();
    Ok(match limit {
        Some(n) => all.into_iter().take(n).collect(),
        None => all,
    })
}

pub fn append(entry: HistoryEntry, max: usize) -> Result<()> {
    let _guard = lock().lock().unwrap();
    let mut all = read_all();
    all.insert(0, entry);
    let max = max.max(1);
    all.truncate(max);
    write_all(&all)
}

pub fn delete(id: &str) -> Result<()> {
    let _guard = lock().lock().unwrap();
    let mut all = read_all();
    all.retain(|e| e.id != id);
    write_all(&all)
}

pub fn find_by_path(path: &str) -> Option<HistoryEntry> {
    read_all().into_iter().find(|e| e.path == path)
}

/// Replace an entry by id, or insert it at the front if it is new.
pub fn replace(entry: HistoryEntry) -> Result<()> {
    let _guard = lock().lock().unwrap();
    let mut all = read_all();
    match all.iter_mut().find(|e| e.id == entry.id) {
        Some(slot) => *slot = entry,
        None => all.insert(0, entry),
    }
    write_all(&all)
}

pub fn clear() -> Result<()> {
    let _guard = lock().lock().unwrap();
    write_all(&[])
}
