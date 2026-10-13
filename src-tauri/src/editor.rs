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
    // Skip the taskbar: this is a transient quick-edit dialog. Tiling
    // compositors (Hyprland, Sway, i3) don't float it on their own — the
    // README documents a one-line float rule matching "ShareV Editor".
    let mut builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("index.html".into()))
        .title("ShareV Editor")
        .inner_size(880.0, 620.0)
        .min_inner_size(620.0, 440.0)
        .resizable(true)
        .fullscreen(false)
        .decorations(true)
        .skip_taskbar(true)
        .shadow(true);

    // Center the window on the current monitor.
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let monitor_size = monitor.size();
        let window_width = 880.0;
        let window_height = 620.0;
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

    // Tiling compositors would tile this quick-edit dialog. Ask Hyprland to
    // float it instead — fully automatic, no user rules needed. Anything
    // missing (not on Hyprland, no hyprctl) just keeps the tiling behavior.
    float_on_hyprland(std::process::id());

    Ok(label)
}

/// Best-effort: float our freshly opened editor window on Hyprland.
fn float_on_hyprland(pid: u32) {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return;
    }
    std::thread::spawn(move || {
        for _ in 0..20 {
            if try_float_editor(pid) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        log::warn!("could not float the editor window on Hyprland; it will tile");
    });
}

/// Find our still-tiled "ShareV Editor" window and toggle it floating.
/// Returns true once the window is floating (or floating was requested).
fn try_float_editor(pid: u32) -> bool {
    let output = std::process::Command::new("hyprctl")
        .args(["clients", "-j"])
        .output()
        .ok();
    let Some(output) = output else { return false };
    let clients: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(clients) => clients,
        Err(_) => return false,
    };
    let Some(clients) = clients.as_array() else {
        return false;
    };
    // Newest first: an older editor window may already exist.
    let target = clients.iter().rev().find(|client| {
        client.get("pid").and_then(|p| p.as_u64()) == Some(u64::from(pid))
            && client.get("title").and_then(|t| t.as_str()) == Some("ShareV Editor")
            && client.get("floating").and_then(|f| f.as_bool()) == Some(false)
    });
    let Some(target) = target else {
        // No tiled editor window of ours: either not mapped yet (keep
        // polling) or every one of ours already floats (done).
        let ours = clients.iter().filter(|client| {
            client.get("pid").and_then(|p| p.as_u64()) == Some(u64::from(pid))
                && client.get("title").and_then(|t| t.as_str()) == Some("ShareV Editor")
        });
        let mut ours = ours.peekable();
        if ours.peek().is_none() {
            return false;
        }
        return !ours.any(|client| {
            client.get("floating").and_then(|f| f.as_bool()) == Some(false)
        });
    };
    let Some(address) = target.get("address").and_then(|a| a.as_str()) else {
        return false;
    };
    // Hyprland ≥0.55 speaks Lua dispatchers over hyprctl; the whole expression
    // is a single argument.
    let expression = format!(
        "hl.dsp.window.float({{ action = \"set\", window = \"address:{address}\" }})"
    );
    std::process::Command::new("hyprctl")
        .args(["dispatch", &expression])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Look up the image associated with an editor window label.
pub fn info(label: &str) -> Option<EditorInfo> {
    registry().lock().unwrap().get(label).cloned()
}
