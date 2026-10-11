use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::paths;

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
    serde_json::from_str(&text).unwrap_or_default()
}

fn write_all(entries: &[HistoryEntry]) -> Result<()> {
    let path = paths::history_file();
    paths::ensure_parent(&path)?;
    let text = serde_json::to_string_pretty(entries)?;
    std::fs::write(path, text)?;
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
    let mut all = read_all();
    all.insert(0, entry);
    let max = max.max(1);
    all.truncate(max);
    write_all(&all)
}

pub fn delete(id: &str) -> Result<()> {
    let mut all = read_all();
    all.retain(|e| e.id != id);
    write_all(&all)
}

pub fn find_by_path(path: &str) -> Option<HistoryEntry> {
    read_all().into_iter().find(|e| e.path == path)
}

/// Replace an entry by id, or insert it at the front if it is new.
pub fn replace(entry: HistoryEntry) -> Result<()> {
    let mut all = read_all();
    match all.iter_mut().find(|e| e.id == entry.id) {
        Some(slot) => *slot = entry,
        None => all.insert(0, entry),
    }
    write_all(&all)
}

pub fn clear() -> Result<()> {
    write_all(&[])
}
