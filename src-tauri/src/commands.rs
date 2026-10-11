use std::path::Path;
use std::process::Stdio;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::capture::{self, CaptureKind, Geometry};
use crate::config::{Config, ImageFormat};
use crate::error::{Error, Result};
use crate::pipeline::{self, CaptureResult};
use crate::record;
use crate::{clipboard, history, paths, AppState};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> Result<()> {
    config.save()?;
    *state.config.lock().unwrap() = config.clone();
    crate::shortcuts::reload(&app, &config);
    Ok(())
}

// ---------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn capture(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    geometry: Option<Geometry>,
) -> Result<CaptureResult> {
    let capture_kind = CaptureKind::parse(&kind)?;
    let config = state.config.lock().unwrap().clone();

    // Hide our own window first so it neither appears in the shot nor counts as
    // the focused window, then give the compositor a moment to settle.
    let window = app.get_webview_window("main");
    if let Some(window) = &window {
        let _ = window.hide();
        tokio::time::sleep(std::time::Duration::from_millis(220)).await;
    }

    let outcome = pipeline::perform(&config, capture_kind, geometry).await;

    if let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }

    let result = outcome?;
    let _ = app.emit("capture-completed", &result);
    if config.show_notification {
        notify_success(&app, &result);
    }
    Ok(result)
}

#[tauri::command]
pub async fn capture_region(app: AppHandle, state: State<'_, AppState>) -> Result<CaptureResult> {
    let config = state.config.lock().unwrap().clone();
    let result = pipeline::perform(&config, CaptureKind::Region, None).await?;
    let _ = app.emit("capture-completed", &result);
    if config.show_notification {
        notify_success(&app, &result);
    }
    Ok(result)
}

/// Shared entry point for the tray menu and global hotkeys.
pub async fn capture_and_notify(app: AppHandle, kind: CaptureKind) {
    let config = app.state::<AppState>().config.lock().unwrap().clone();
    match pipeline::perform(&config, kind, None).await {
        Ok(result) => {
            let _ = app.emit("capture-completed", &result);
            if config.show_notification {
                notify_success(&app, &result);
            }
        }
        Err(Error::Cancelled) => {}
        Err(err) => {
            log::warn!("capture failed: {err}");
            if config.show_notification {
                notify_error(&app, &err.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Annotation editor
// ---------------------------------------------------------------------------

/// Read an image file as a `data:` URL the webview can draw onto a canvas
/// without tainting it.
#[tauri::command]
pub fn read_image_base64(path: String) -> Result<String> {
    let bytes = std::fs::read(&path)?;
    Ok(format!(
        "data:{};base64,{}",
        mime_from_path(&path),
        STANDARD.encode(bytes)
    ))
}

/// Apply annotated image data (a PNG data URL) to an existing capture: encode
/// in the configured format, overwrite the file, update history and clipboard.
#[tauri::command]
pub fn apply_edits(
    state: State<'_, AppState>,
    path: String,
    data: String,
) -> Result<CaptureResult> {
    let config = state.config.lock().unwrap().clone();
    let png = decode_data_url(&data)?;
    let image = image::load_from_memory(&png)?.to_rgba8();

    let format = config.image_format;
    let encoded = pipeline::encode(&image, format, config.jpeg_quality)?;
    std::fs::write(&path, &encoded)?;

    let width = image.width();
    let height = image.height();
    let bytes = encoded.len() as u64;

    let mut copied = false;
    if config.copy_to_clipboard && clipboard::copy_image(Path::new(&path), format).is_ok() {
        copied = true;
    }

    let mut entry = history::find_by_path(&path).unwrap_or_else(|| history::HistoryEntry {
        id: crate::id::new_id(),
        path: path.clone(),
        file_name: Path::new(&path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        kind: "edited".into(),
        width,
        height,
        bytes,
        backend: "editor".into(),
        created_at: chrono::Local::now().to_rfc3339(),
        url: None,
    });
    entry.width = width;
    entry.height = height;
    entry.bytes = bytes;
    history::replace(entry.clone())?;

    Ok(CaptureResult {
        id: entry.id,
        path: entry.path,
        file_name: entry.file_name,
        kind: entry.kind,
        width,
        height,
        bytes,
        backend: entry.backend,
        created_at: entry.created_at,
        copied,
    })
}

/// Copy annotated image data (a PNG data URL) to the clipboard without saving.
#[tauri::command]
pub fn copy_edits(data: String) -> Result<()> {
    let png = decode_data_url(&data)?;
    clipboard::copy_bytes(&png, "image/png")
}

fn mime_from_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        _ => "image/png",
    }
}

fn decode_data_url(data: &str) -> Result<Vec<u8>> {
    let payload = data.split_once(',').map(|(_, rest)| rest).unwrap_or(data);
    STANDARD
        .decode(payload.trim())
        .map_err(|err| Error::msg(format!("invalid image data: {err}")))
}

// ---------------------------------------------------------------------------
// Upload / share
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadOutcome {
    /// The URL returned by the destination.
    pub url: String,
    /// The formatted link that was copied to the clipboard.
    pub link: String,
    /// Whether copying the link succeeded.
    pub copied: bool,
    /// Destination that handled the upload ("imgur", "http", "command").
    pub uploader: String,
}

/// Upload a saved capture to the configured destination and copy the link.
#[tauri::command]
pub async fn upload_path(state: State<'_, AppState>, path: String) -> Result<UploadOutcome> {
    let config = state.config.lock().unwrap().clone();
    let url = crate::upload::upload(&config, Path::new(&path)).await?;
    let link = config.upload.copy_format.format(&url);
    let copied = clipboard::copy_text(&link).is_ok();

    // Record the URL on the matching history entry.
    if let Some(mut entry) = history::find_by_path(&path) {
        entry.url = Some(url.clone());
        if let Err(err) = history::replace(entry) {
            log::warn!("could not record upload url: {err}");
        }
    }

    Ok(UploadOutcome {
        url,
        link,
        copied,
        uploader: config.upload.uploader.as_str().to_string(),
    })
}

// ---------------------------------------------------------------------------
// Screen recording
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    pub path: String,
    pub kind: String,
    pub format: String,
    pub backend: String,
    pub started_at: String,
    pub width: u32,
    pub height: u32,
}

impl RecordingInfo {
    fn from_active(active: &record::ActiveRecording) -> Self {
        Self {
            path: active.output.to_string_lossy().into_owned(),
            kind: active.kind.as_str().to_string(),
            format: active.format.as_str().to_string(),
            backend: active.backend.clone(),
            started_at: active.started_at.clone(),
            width: active.width,
            height: active.height,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingResult {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub kind: String,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub duration_ms: u64,
    pub backend: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub monitor: bool,
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
) -> Result<RecordingInfo> {
    let config = state.config.lock().unwrap().clone();
    let capture_kind = CaptureKind::parse(&kind)?;

    if state.recorder.lock().unwrap().is_some() {
        return Err(Error::msg("a recording is already in progress"));
    }

    let backend = record::choose_backend()?;

    // Hide our own window while the user picks a region/window.
    let window = app.get_webview_window("main");
    let interactive = matches!(capture_kind, CaptureKind::Region | CaptureKind::Window);
    if interactive {
        if let Some(window) = &window {
            let _ = window.hide();
        }
        tokio::time::sleep(std::time::Duration::from_millis(220)).await;
    }

    let geometry = record::resolve_geometry(capture_kind, backend).await;

    if let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let geometry = geometry?;

    let request = record::build_request(&config, capture_kind, geometry);
    let active = record::start(request)?;
    let info = RecordingInfo::from_active(&active);
    *state.recorder.lock().unwrap() = Some(active);
    let _ = app.emit("recording-started", &info);
    Ok(info)
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle, state: State<'_, AppState>) -> Result<RecordingResult> {
    let config = state.config.lock().unwrap().clone();
    let active = state.recorder.lock().unwrap().take();
    let Some(active) = active else {
        return Err(Error::msg("no recording is in progress"));
    };

    let result = stop_and_store(active, &config).await?;
    let _ = app.emit("recording-stopped", &result);
    Ok(result)
}

/// Start a recording and return once the user interrupts the process (Ctrl-C).
/// Used by the headless CLI so it can be bound to a compositor shortcut.
pub async fn record_until_interrupt(config: &Config, kind: CaptureKind) -> Result<RecordingResult> {
    let backend = record::choose_backend()?;
    let geometry = record::resolve_geometry(kind, backend).await?;
    let active = record::start(record::build_request(config, kind, geometry))?;
    tokio::signal::ctrl_c()
        .await
        .map_err(|err| Error::msg(err.to_string()))?;
    stop_and_store(active, config).await
}

/// Signal the encoder to stop, verify the output, run post-processing and add a
/// history entry. Shared by the Tauri command and the CLI.
async fn stop_and_store(
    active: record::ActiveRecording,
    config: &Config,
) -> Result<RecordingResult> {
    let log_path = active.log_path.clone();
    let record_path = active.record_path.clone();
    let started = active.started;

    // Signal + wait off the async runtime, then keep the metadata.
    let active = tokio::task::spawn_blocking(move || -> Result<record::ActiveRecording> {
        let mut active = active;
        record::stop(&mut active)?;
        Ok(active)
    })
    .await
    .map_err(|err| Error::msg(err.to_string()))??;

    if !record_path.exists() {
        let tail = record::log_tail(&log_path);
        let _ = std::fs::remove_file(&log_path);
        return Err(match tail {
            Some(tail) => Error::msg(format!("recording failed: {tail}")),
            None => Error::msg("recording produced no file"),
        });
    }

    record::finalize(&active).await?;

    let path = active.output.clone();
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let duration_ms = started.elapsed().as_millis() as u64;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let id = crate::id::new_id();
    let entry = history::HistoryEntry {
        id: id.clone(),
        path: path.to_string_lossy().into_owned(),
        file_name: file_name.clone(),
        kind: "recording".into(),
        width: active.width,
        height: active.height,
        bytes,
        backend: active.backend.clone(),
        created_at: active.started_at.clone(),
        url: None,
    };
    history::append(entry, config.max_history)?;

    Ok(RecordingResult {
        id,
        path: path.to_string_lossy().into_owned(),
        file_name,
        kind: "recording".into(),
        format: active.format.as_str().to_string(),
        width: active.width,
        height: active.height,
        bytes,
        duration_ms,
        backend: active.backend,
        created_at: active.started_at,
    })
}

#[tauri::command]
pub fn recording_status(state: State<'_, AppState>) -> Option<RecordingInfo> {
    let guard = state.recorder.lock().unwrap();
    guard.as_ref().map(RecordingInfo::from_active)
}

/// List PulseAudio/PipeWire source devices (monitors and microphones).
#[tauri::command]
pub async fn list_audio_devices() -> Result<Vec<AudioDevice>> {
    if capture::which("pactl").is_none() {
        return Ok(Vec::new());
    }
    let output = tokio::process::Command::new("pactl")
        .args(["-f", "json", "list", "sources"])
        .output()
        .await
        .map_err(Error::Io)?;
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or(serde_json::Value::Null);
    let mut devices = Vec::new();
    if let Some(items) = value.as_array() {
        for item in items {
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let description = item
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or(&name)
                .to_string();
            let monitor = name.contains(".monitor");
            devices.push(AudioDevice {
                name,
                description,
                monitor,
            });
        }
    }
    Ok(devices)
}

// ---------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_history(limit: Option<usize>) -> Result<Vec<history::HistoryEntry>> {
    history::list(limit)
}

#[tauri::command]
pub fn delete_history(id: String) -> Result<()> {
    history::delete(&id)
}

#[tauri::command]
pub fn clear_history() -> Result<()> {
    history::clear()
}

// ---------------------------------------------------------------------------
// Files / clipboard
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_path(path: String) -> Result<()> {
    open_with(&path)
}

#[tauri::command]
pub fn reveal_path(path: String) -> Result<()> {
    let target = Path::new(&path);
    let dir = target
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(paths::default_save_dir);
    open_with(&dir.to_string_lossy())
}

#[tauri::command]
pub fn copy_file_to_clipboard(path: String) -> Result<()> {
    let format = match Path::new(&path)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => ImageFormat::Jpeg,
        Some("webp") => ImageFormat::Webp,
        _ => ImageFormat::Png,
    };
    clipboard::copy_image(Path::new(&path), format)
}

#[tauri::command]
pub fn copy_text(text: String) -> Result<()> {
    clipboard::copy_text(&text)
}

#[tauri::command]
pub async fn pick_directory(app: AppHandle) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |folder| {
        let _ = tx.send(folder);
    });
    let folder = rx.await.ok().flatten()?;
    folder.into_path().ok().map(|p| p.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub version: String,
    pub session_type: String,
    pub wayland: bool,
    pub x11: bool,
    pub desktop: String,
    pub active_backend: Option<String>,
    pub backends: Vec<capture::BackendInfo>,
    pub save_directory: String,
    pub config_path: String,
    pub history_path: String,
}

pub fn diagnostics_info(state: &AppState) -> Diagnostics {
    let config = state.config.lock().unwrap().clone();
    Diagnostics {
        version: env!("CARGO_PKG_VERSION").to_string(),
        session_type: crate::platform::display::detect().as_str().to_string(),
        wayland: std::env::var_os("WAYLAND_DISPLAY").is_some(),
        x11: std::env::var_os("DISPLAY").is_some(),
        desktop: std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into()),
        active_backend: capture::select_backend(&config.capture_backend)
            .ok()
            .map(|b| b.id().to_string()),
        backends: capture::backend_infos(),
        save_directory: config.resolved_save_dir().to_string_lossy().into_owned(),
        config_path: paths::config_file().to_string_lossy().into_owned(),
        history_path: paths::history_file().to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub fn diagnostics(state: State<'_, AppState>) -> Diagnostics {
    diagnostics_info(&state)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn open_with(target: &str) -> Result<()> {
    std::process::Command::new("xdg-open")
        .arg(target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(Error::Io)
}

fn notify_success(app: &AppHandle, result: &CaptureResult) {
    let _ = app
        .notification()
        .builder()
        .title("ShareV")
        .body(format!(
            "Saved {} ({}x{})",
            result.file_name, result.width, result.height
        ))
        .show();
}

fn notify_error(app: &AppHandle, message: &str) {
    let _ = app
        .notification()
        .builder()
        .title("ShareV capture failed")
        .body(message)
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mime_from_extension() {
        assert_eq!(mime_from_path("/tmp/shot.PNG"), "image/png");
        assert_eq!(mime_from_path("/tmp/shot.png"), "image/png");
        assert_eq!(mime_from_path("/tmp/shot.jpg"), "image/jpeg");
        assert_eq!(mime_from_path("/tmp/shot.jpeg"), "image/jpeg");
        assert_eq!(mime_from_path("/tmp/shot.webp"), "image/webp");
        assert_eq!(mime_from_path("/tmp/shot"), "image/png");
    }

    #[test]
    fn decodes_data_urls_and_raw_base64() {
        let payload = STANDARD.encode(b"annotated bytes");
        let decoded =
            decode_data_url(&format!("data:image/png;base64,{payload}")).expect("data url");
        assert_eq!(decoded, b"annotated bytes");

        let raw = decode_data_url(&payload).expect("raw base64");
        assert_eq!(raw, b"annotated bytes");
    }

    #[test]
    fn rejects_garbage_image_data() {
        assert!(decode_data_url("data:image/png;base64,!!!not-base64!!!").is_err());
    }
}
