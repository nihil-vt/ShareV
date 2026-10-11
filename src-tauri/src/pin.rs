//! Pinned captures ("pin to screen").
//!
//! A pin is a small, borderless, always-on-top webview window that renders a
//! single captured image. The image path is kept in an in-process registry keyed
//! by the window label so the frontend can look it up without encoding the path
//! into the window URL.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::capture::Geometry;
use crate::error::{Error, Result};

/// Data the pin window needs to render itself.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinInfo {
    pub path: String,
    pub width: u32,
    pub height: u32,
}

fn registry() -> &'static Mutex<HashMap<String, PinInfo>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, PinInfo>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Create a pinned window for `path`, positioned over `geometry` when known.
/// Returns the new window's label.
pub fn pin(
    app: &AppHandle,
    path: &str,
    width: u32,
    height: u32,
    geometry: Option<Geometry>,
) -> Result<String> {
    let label = format!("pin-{}", crate::id::new_id());
    registry().lock().unwrap().insert(
        label.clone(),
        PinInfo {
            path: path.to_string(),
            width,
            height,
        },
    );

    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("ShareV Pin")
        .inner_size(width.max(32) as f64, height.max(32) as f64)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .transparent(true)
        .shadow(false);

    if let Some(g) = geometry {
        builder = builder.position(g.x as f64, g.y as f64);
    }

    let window = builder
        .build()
        .map_err(|err| Error::msg(format!("could not pin window: {err}")))?;

    let cleanup_label = label.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Destroyed) {
            registry().lock().unwrap().remove(&cleanup_label);
        }
    });

    Ok(label)
}

/// Look up the image associated with a pin window label.
pub fn info(label: &str) -> Option<PinInfo> {
    registry().lock().unwrap().get(label).cloned()
}
