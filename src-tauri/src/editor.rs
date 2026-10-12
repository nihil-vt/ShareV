//! Standalone annotation editor window.
//!
//! The editor opens in its own small webview window (like ShareX), separate
//! from the main app window. The image path is kept in an in-process registry
//! keyed by window label so the frontend can look it up.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::error::{Error, Result};

/// Data the editor window needs to render itself.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

fn registry() -> &'static Mutex<HashMap<String, EditorInfo>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, EditorInfo>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Create an editor window for `path`. Returns the new window's label.
pub fn open_editor(app: &AppHandle, path: &str) -> Result<String> {
    let label = format!("editor-{}", crate::id::new_id());
    let (width, height) = image::image_dimensions(path).unwrap_or((0, 0));

    registry().lock().unwrap().insert(
        label.clone(),
        EditorInfo {
            path: path.to_string(),
            width,
            height,
        },
    );

    // Editor window: smaller than main, resizable, not fullscreen, no snap.
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("ShareV Editor")
        .inner_size(900.0, 640.0)
        .min_inner_size(600.0, 400.0)
        .resizable(true)
        .fullscreen(false)
        .decorations(true)
        .shadow(true);

    // Center the window on the current monitor.
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let monitor_size = monitor.size();
        let window_width = 900.0;
        let window_height = 640.0;
        let x = (monitor_size.width as f64 - window_width) / 2.0;
        let y = (monitor_size.height as f64 - window_height) / 2.0;
        builder = builder.position(x, y);
    }

    let window = builder
        .build()
        .map_err(|err| Error::msg(format!("could not open editor window: {err}")))?;

    let cleanup_label = label.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            registry().lock().unwrap().remove(&cleanup_label);
        }
    });

    Ok(label)
}

/// Look up the image associated with an editor window label.
pub fn info(label: &str) -> Option<EditorInfo> {
    registry().lock().unwrap().get(label).cloned()
}
