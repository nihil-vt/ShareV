use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

use crate::auto::{AutoCaptureInfo, AutoCaptureSession};
use crate::capture::{self, CaptureKind, Geometry};
use crate::config::{Config, ImageFormat, Task};
use crate::error::{Error, Result};
use crate::pipeline::{self, CaptureResult};
use crate::record;
use crate::{AppState, clipboard, history, paths, tasks};

/// Sleep for the configured pre-capture delay.
async fn apply_delay(seconds: Option<u32>) {
    let seconds = seconds.unwrap_or(0);
    if seconds > 0 {
        tokio::time::sleep(Duration::from_secs(seconds as u64)).await;
    }
}

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
    monitor: Option<String>,
    window_id: Option<String>,
    delay_seconds: Option<u32>,
) -> Result<CaptureResult> {
    let capture_kind = CaptureKind::parse(&kind)?;
    let config = state.config.lock().unwrap().clone();

    apply_delay(delay_seconds).await;

    // Hide our own window so it neither appears in the shot nor counts as the
    // focused window, then give the compositor a moment to settle.
    let window = app.get_webview_window("main");
    if let Some(window) = &window {
        let _ = window.hide();
        tokio::time::sleep(std::time::Duration::from_millis(220)).await;
    }

    let outcome = pipeline::grab(&config, capture_kind, geometry, monitor, window_id).await;

    if let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }

    let draft = outcome?;
    finish_capture(&app, &config, draft).await
}

/// Capture a scrolling region: the user selects an area, scrolls while ShareV
/// captures frames, and the frames are stitched into one tall image.
#[tauri::command]
pub async fn capture_scrolling(
    app: AppHandle,
    state: State<'_, AppState>,
    delay_seconds: Option<u32>,
) -> Result<CaptureResult> {
    let config = state.config.lock().unwrap().clone();
    apply_delay(delay_seconds).await;

    let window = app.get_webview_window("main");
    if let Some(window) = &window {
        let _ = window.hide();
        tokio::time::sleep(std::time::Duration::from_millis(220)).await;
    }

    let notifier = app.clone();
    let outcome = pipeline::grab_scrolling(&config, move |_geometry| {
        notify_info(
            &notifier,
            "Scrolling capture",
            "Now scroll through the content — capture stops automatically when it stops moving.",
        );
    })
    .await;

    if let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }

    let draft = outcome?;
    finish_capture(&app, &config, draft).await
}

/// Run the task chain, announce the result and refresh the tray.
async fn finish_capture(
    app: &AppHandle,
    config: &Config,
    draft: pipeline::CaptureDraft,
) -> Result<CaptureResult> {
    let result = tasks::run(Some(app), config, draft).await?;
    let _ = app.emit("capture-completed", &result);
    crate::tray::refresh(app);
    Ok(result)
}

// ---------------------------------------------------------------------------
// Auto capture
// ---------------------------------------------------------------------------

/// The shared auto-capture loop.
///
/// Runs until the session is asked to stop, its count limit is reached, or its
/// time limit expires. When `app` is `None` (CLI) each saved path is printed to
/// stdout.
pub async fn run_auto_capture(
    app: Option<AppHandle>,
    config: Config,
    geometry: Option<Geometry>,
    monitor: Option<String>,
    session: AutoCaptureSession,
) -> Result<u32> {
    let mut previous: Option<image::RgbaImage> = None;
    let mut saved = 0u32;

    loop {
        if session.is_stopping() {
            break;
        }
        if session.config.max_count > 0 && saved >= session.config.max_count {
            break;
        }
        if session.config.max_duration_seconds > 0
            && session.started.elapsed().as_secs() >= session.config.max_duration_seconds
        {
            break;
        }

        let interval = Duration::from_millis(session.config.interval_ms.max(1));
        if session.wait_interval(interval).await {
            break;
        }

        let draft = match pipeline::grab_locked(&config, geometry, monitor.clone()).await {
            Ok(draft) => draft,
            Err(Error::Cancelled) => break,
            Err(err) => {
                log::warn!("auto capture failed: {err}");
                if let Some(app) = &app {
                    notify_error(app, &err.to_string());
                }
                continue;
            }
        };

        // Only diff frames (and keep a copy) when change detection is enabled.
        if session.config.change_detection {
            let changed = match &previous {
                None => true,
                Some(prev) => {
                    crate::auto::frame_changed(prev, &draft.image, session.config.change_threshold)
                }
            };
            previous = Some(draft.image.clone());
            if !changed {
                continue;
            }
        }

        saved += 1;
        session.count.store(saved, Ordering::SeqCst);

        match tasks::run(app.as_ref(), &config, draft).await {
            Ok(result) => {
                if let Some(app) = &app {
                    let _ = app.emit("capture-completed", &result);
                    crate::tray::refresh(app);
                } else {
                    println!("{}", result.path);
                }
            }
            Err(err) => {
                log::warn!("auto capture task failed: {err}");
                if let Some(app) = &app {
                    notify_error(app, &err.to_string());
                }
            }
        }
    }

    Ok(saved)
}

/// Clear the auto-capture slot if it still holds `session`.
fn release_auto_slot(state: &State<'_, AppState>, session: &AutoCaptureSession) {
    let mut guard = state.auto.lock().unwrap();
    if guard
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(&current.stop, &session.stop))
    {
        *guard = None;
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn start_auto_capture(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    interval_ms: Option<u64>,
    change_detection: Option<bool>,
    change_threshold: Option<f32>,
    max_count: Option<u32>,
    max_duration_seconds: Option<u64>,
    monitor: Option<String>,
    window_id: Option<String>,
) -> Result<AutoCaptureInfo> {
    let capture_kind = CaptureKind::parse(&kind)?;
    if capture_kind == CaptureKind::Scrolling {
        return Err(Error::msg(
            "auto capture does not support the 'scrolling' target",
        ));
    }

    // Start from the saved settings, then apply any per-run overrides.
    let mut config = state.config.lock().unwrap().clone();
    if let Some(value) = interval_ms {
        config.auto.interval_ms = value.max(1);
    }
    if let Some(value) = change_detection {
        config.auto.change_detection = value;
    }
    if let Some(value) = change_threshold {
        config.auto.change_threshold = value.clamp(0.0, 1.0);
    }
    if let Some(value) = max_count {
        config.auto.max_count = value;
    }
    if let Some(value) = max_duration_seconds {
        config.auto.max_duration_seconds = value;
    }

    let session = AutoCaptureSession::new(config.auto.clone(), capture_kind);

    // Reserve the slot before interacting with the user so a second start
    // (double click, hotkey or tray action) cannot race us into two sessions.
    {
        let mut guard = state.auto.lock().unwrap();
        if guard.is_some() {
            return Err(Error::msg("an auto capture is already running"));
        }
        *guard = Some(session.clone());
    }

    // Resolve a fixed region/window geometry once; the region is locked for the
    // whole session so every frame lines up.
    let interactive = matches!(capture_kind, CaptureKind::Region | CaptureKind::Window);
    let window = app.get_webview_window("main");
    if interactive {
        if let Some(window) = &window {
            let _ = window.hide();
        }
        tokio::time::sleep(Duration::from_millis(220)).await;
    }

    let geometry = match capture_kind {
        CaptureKind::Fullscreen | CaptureKind::Scrolling => None,
        CaptureKind::Region | CaptureKind::Window => {
            match pipeline::grab(
                &config,
                capture_kind,
                None,
                monitor.clone(),
                window_id.clone(),
            )
            .await
            {
                Ok(draft) => draft.geometry,
                Err(err) => {
                    if interactive && let Some(window) = &window {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    release_auto_slot(&state, &session);
                    return Err(err);
                }
            }
        }
    };

    if interactive && let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }

    if matches!(capture_kind, CaptureKind::Region | CaptureKind::Window) && geometry.is_none() {
        release_auto_slot(&state, &session);
        return Err(Error::msg("could not determine the selected region"));
    }

    let info = session.info();
    let _ = app.emit("auto-capture-started", &info);

    let loop_app = app.clone();
    let loop_session = session.clone();
    let loop_config = config.clone();
    tauri::async_runtime::spawn(async move {
        let result = run_auto_capture(
            Some(loop_app.clone()),
            loop_config,
            geometry,
            monitor,
            loop_session.clone(),
        )
        .await;

        // Clear the session only if it is still the active one.
        {
            let state = loop_app.state::<AppState>();
            release_auto_slot(&state, &loop_session);
        }

        if let Err(err) = &result {
            log::warn!("auto capture failed: {err}");
            notify_error(&loop_app, &err.to_string());
        }
        let _ = loop_app.emit("auto-capture-stopped", &loop_session.info());
    });

    Ok(info)
}

#[tauri::command]
pub fn stop_auto_capture(state: State<'_, AppState>) -> Result<AutoCaptureInfo> {
    let session = state.auto.lock().unwrap().clone();
    let Some(session) = session else {
        return Err(Error::msg("no auto capture is running"));
    };
    session.request_stop();
    Ok(session.info())
}

#[tauri::command]
pub fn auto_capture_status(state: State<'_, AppState>) -> Option<AutoCaptureInfo> {
    state.auto.lock().unwrap().as_ref().map(|s| s.info())
}

/// Shared entry point for the tray menu and global hotkeys.
pub async fn capture_and_notify(app: AppHandle, kind: CaptureKind) {
    let config = app.state::<AppState>().config.lock().unwrap().clone();

    apply_delay(Some(config.capture_delay_seconds)).await;

    let window = app.get_webview_window("main");
    let was_visible = window
        .as_ref()
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    if was_visible {
        if let Some(window) = &window {
            let _ = window.hide();
        }
        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
    }

    let outcome = if kind == CaptureKind::Scrolling {
        let notifier = app.clone();
        pipeline::grab_scrolling(&config, move |_geometry| {
            notify_info(
                &notifier,
                "Scrolling capture",
                "Now scroll through the content — capture stops automatically when it stops moving.",
            );
        })
        .await
    } else {
        pipeline::grab(&config, kind, None, None, None).await
    };

    if was_visible && let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }

    match outcome {
        Ok(draft) => match finish_capture(&app, &config, draft).await {
            Ok(_) => {}
            Err(err) => {
                log::warn!("capture failed: {err}");
                notify_error(&app, &err.to_string());
            }
        },
        Err(Error::Cancelled) => {}
        Err(err) => {
            log::warn!("capture failed: {err}");
            notify_error(&app, &err.to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// Windows & monitors
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_windows() -> Result<Vec<crate::wm::WindowInfo>> {
    crate::wm::list_windows().await
}

#[tauri::command]
pub async fn list_monitors() -> Result<Vec<crate::wm::MonitorInfo>> {
    crate::wm::list_monitors().await
}

// ---------------------------------------------------------------------------
// Pinned captures
// ---------------------------------------------------------------------------

/// Pin an existing image file to the screen.
#[tauri::command]
pub fn pin_capture(app: AppHandle, path: String) -> Result<String> {
    if !Path::new(&path).exists() {
        return Err(Error::msg("that file no longer exists"));
    }
    let (width, height) = image::image_dimensions(&path).unwrap_or((0, 0));
    crate::pin::pin(&app, &path, width, height, None)
}

/// Data for a pin window to render itself (looked up by window label).
#[tauri::command]
pub fn pin_info(window: tauri::WebviewWindow) -> Option<crate::pin::PinInfo> {
    crate::pin::info(window.label())
}

/// Pin the most recent capture in the history.
#[tauri::command]
pub fn pin_last(app: AppHandle) -> Result<Option<String>> {
    let Some(entry) = history::list(Some(1))?.into_iter().next() else {
        return Ok(None);
    };
    if !Path::new(&entry.path).exists() {
        return Err(Error::msg("the last capture is no longer on disk"));
    }
    let (width, height) =
        image::image_dimensions(&entry.path).unwrap_or((entry.width, entry.height));
    crate::pin::pin(&app, &entry.path, width, height, None).map(Some)
}

// ---------------------------------------------------------------------------
// Annotation editor
// ---------------------------------------------------------------------------

/// Open the annotation editor in a standalone window for `path`.
#[tauri::command]
pub fn open_editor(app: AppHandle, path: String) -> Result<String> {
    if !Path::new(&path).exists() {
        return Err(Error::msg("that file no longer exists"));
    }
    crate::editor::open_editor(&app, &path)
}

/// Data for an editor window to render itself (looked up by window label).
#[tauri::command]
pub fn editor_info(window: tauri::WebviewWindow) -> Option<crate::editor::EditorInfo> {
    crate::editor::info(window.label())
}

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

    // Preserve the file's own format/extension rather than the configured one,
    // otherwise we'd write PNG bytes into a `.jpg` and break it on next open.
    let format = ImageFormat::from_path(Path::new(&path)).unwrap_or(config.image_format);
    let encoded = pipeline::encode(&image, format, config.jpeg_quality)?;
    std::fs::write(&path, &encoded)?;

    let width = image.width();
    let height = image.height();
    let bytes = encoded.len() as u64;

    let mut copied = false;
    if config.has_capture_task(Task::CopyImage)
        && clipboard::copy_image(Path::new(&path), format).is_ok()
    {
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
        url: entry.url,
        open_editor: false,
    })
}

/// Copy annotated image data (a PNG data URL) to the clipboard without saving.
#[tauri::command]
pub fn copy_edits(data: String) -> Result<()> {
    let png = decode_data_url(&data)?;
    clipboard::copy_bytes(&png, "image/png")
}

// ---------------------------------------------------------------------------
// OCR
// ---------------------------------------------------------------------------

/// Check if Tesseract is available and list installed languages.
#[tauri::command]
pub async fn ocr_status() -> serde_json::Value {
    let (available, langs) = crate::ocr::check_available().await;
    serde_json::json!({ "available": available, "languages": langs })
}

/// Run OCR on an image file and return the recognized text.
#[tauri::command]
pub async fn ocr_recognize(path: String, language: String) -> Result<String> {
    crate::ocr::recognize(&path, &language).await
}

/// Apply image effects to a file and overwrite it.
#[tauri::command]
pub fn apply_effects(
    state: State<'_, AppState>,
    path: String,
    effects: crate::effects::EffectsConfig,
) -> Result<()> {
    let config = state.config.lock().unwrap().clone();
    let img = image::open(&path)
        .map_err(|err| crate::error::Error::msg(format!("could not open image: {err}")))?;
    let result = crate::effects::apply_effects(&img, &effects);
    // Re-encode through the pipeline so the bytes match the file extension
    // (and transparent effects flatten correctly for JPEG).
    let format = ImageFormat::from_path(Path::new(&path)).unwrap_or(config.image_format);
    let encoded = pipeline::encode(&result.to_rgba8(), format, config.jpeg_quality)?;
    std::fs::write(&path, &encoded)?;
    Ok(())
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

/// Perform the upload itself and record the URL on the history entry.
async fn do_upload(config: &Config, path: &str) -> Result<String> {
    let url = crate::upload::upload(config, Path::new(path)).await?;
    if let Some(mut entry) = history::find_by_path(path) {
        entry.url = Some(url.clone());
        if let Err(err) = history::replace(entry) {
            log::warn!("could not record upload url: {err}");
        }
    }
    Ok(url)
}

fn upload_outcome(
    config: &Config,
    path: &str,
    url: String,
    link: String,
    copied: bool,
) -> UploadOutcome {
    let _ = path;
    UploadOutcome {
        url,
        link,
        copied,
        uploader: config.upload.uploader.as_str().to_string(),
    }
}

/// Upload a saved capture to the configured destination and run the
/// post-upload tasks.
#[tauri::command]
pub async fn upload_path(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<UploadOutcome> {
    let config = state.config.lock().unwrap().clone();
    let url = do_upload(&config, &path).await?;
    let (link, copied) = tasks::after_upload(Some(&app), &config, &url).await;
    let outcome = upload_outcome(&config, &path, url, link, copied);
    let _ = app.emit("upload-completed", &outcome);
    Ok(outcome)
}

/// Upload the most recent capture in the history (used by the tray).
#[tauri::command]
pub async fn upload_last(app: AppHandle) -> Result<Option<UploadOutcome>> {
    let config = app.state::<AppState>().config.lock().unwrap().clone();
    let Some(entry) = history::list(Some(1))?.into_iter().next() else {
        return Ok(None);
    };
    if !Path::new(&entry.path).exists() {
        return Err(Error::msg("the last capture is no longer on disk"));
    }
    let url = do_upload(&config, &entry.path).await?;
    let (link, copied) = tasks::after_upload(Some(&app), &config, &url).await;
    let outcome = upload_outcome(&config, &entry.path, url, link, copied);
    let _ = app.emit("upload-completed", &outcome);
    Ok(Some(outcome))
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

/// Start a recording for `kind`, applying the configured pre-roll delay and
/// monitor/window target. Shared by the Tauri command, the tray and hotkeys.
/// Clears the "recording is starting" reservation when dropped, so every error
/// path in `start_recording_inner` releases the slot.
struct RecordingReservation<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for RecordingReservation<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

async fn start_recording_inner(
    app: &AppHandle,
    kind: CaptureKind,
    delay_seconds: u32,
    monitor: Option<String>,
    window_id: Option<String>,
) -> Result<RecordingInfo> {
    let config = app.state::<AppState>().config.lock().unwrap().clone();

    // Reserve the slot before the first await so a concurrent start (hotkey,
    // tray or second click) cannot race us into two recordings.
    let state = app.state::<AppState>();
    if state.recorder.lock().unwrap().is_some() {
        return Err(Error::msg("a recording is already in progress"));
    }
    if state
        .recording_pending
        .swap(true, std::sync::atomic::Ordering::SeqCst)
    {
        return Err(Error::msg("a recording is already in progress"));
    }
    let _reservation = RecordingReservation(&state.recording_pending);

    let backend = record::choose_backend()?;

    apply_delay(Some(delay_seconds)).await;

    // Hide our own window while the user picks a region/window.
    let interactive = matches!(
        kind,
        CaptureKind::Region | CaptureKind::Window | CaptureKind::Scrolling
    );
    let window = app.get_webview_window("main");
    if interactive {
        if let Some(window) = &window {
            let _ = window.hide();
        }
        tokio::time::sleep(std::time::Duration::from_millis(220)).await;
    }

    let monitor = monitor.unwrap_or_else(|| config.monitor.clone());
    let geometry = record::resolve_geometry(kind, backend, &monitor, window_id.as_deref()).await;

    if interactive && let Some(window) = &window {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let geometry = geometry?;

    let request = record::build_request(&config, kind, geometry);
    let active = record::start(request)?;
    let info = RecordingInfo::from_active(&active);
    *app.state::<AppState>().recorder.lock().unwrap() = Some(active);
    let _ = app.emit("recording-started", &info);
    crate::tray::refresh(app);
    Ok(info)
}

#[tauri::command]
pub async fn start_recording(
    app: AppHandle,
    kind: String,
    delay_seconds: Option<u32>,
    monitor: Option<String>,
    window_id: Option<String>,
) -> Result<RecordingInfo> {
    let capture_kind = CaptureKind::parse(&kind)?;
    start_recording_inner(
        &app,
        capture_kind,
        delay_seconds.unwrap_or(0),
        monitor,
        window_id,
    )
    .await
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
    crate::tray::refresh(&app);
    Ok(result)
}

/// Start recording if idle, or stop the active recording (used by the tray and
/// the `record-toggle` hotkey action).
pub async fn toggle_recording(app: AppHandle, kind: CaptureKind) {
    let pending = app
        .state::<AppState>()
        .recording_pending
        .load(std::sync::atomic::Ordering::SeqCst);
    let active = app.state::<AppState>().recorder.lock().unwrap().is_some() || pending;
    if active {
        let config = app.state::<AppState>().config.lock().unwrap().clone();
        let active = app.state::<AppState>().recorder.lock().unwrap().take();
        if let Some(active) = active {
            match stop_and_store(active, &config).await {
                Ok(result) => {
                    let _ = app.emit("recording-stopped", &result);
                }
                Err(err) => {
                    log::warn!("recording failed: {err}");
                    notify_error(&app, &err.to_string());
                }
            }
        }
        crate::tray::refresh(&app);
    } else if let Err(err) = start_recording_inner(&app, kind, 0, None, None).await {
        log::warn!("could not start recording: {err}");
        notify_error(&app, &err.to_string());
    }
}

/// Start a recording and return once the user interrupts the process (Ctrl-C).
/// Used by the headless CLI so it can be bound to a compositor shortcut.
pub async fn record_until_interrupt(
    config: &Config,
    kind: CaptureKind,
    delay_seconds: u32,
    monitor: String,
    window_id: Option<String>,
) -> Result<RecordingResult> {
    let backend = record::choose_backend()?;
    apply_delay(Some(delay_seconds)).await;
    let geometry = record::resolve_geometry(kind, backend, &monitor, window_id.as_deref()).await?;
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
    folder
        .into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
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
    /// Which hotkey mechanism is active ("portal", "global-shortcut" or "none").
    pub hotkey_backend: String,
    /// Whether the XDG portal GlobalShortcuts interface is reachable.
    pub portal_shortcuts: bool,
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
        hotkey_backend: crate::shortcuts::active_backend().to_string(),
        portal_shortcuts: crate::shortcuts::portal_available(),
    }
}

#[tauri::command]
pub fn diagnostics(state: State<'_, AppState>) -> Diagnostics {
    diagnostics_info(&state)
}

/// True when running inside a Flatpak sandbox, where Flatpak (not the built-in
/// updater) is responsible for updates.
#[tauri::command]
pub fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists() || std::env::var_os("FLATPAK_ID").is_some()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn open_with(target: &str) -> Result<()> {
    let child = std::process::Command::new("xdg-open")
        .arg(target)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(Error::Io)?;
    // Reap xdg-open on a background thread so it doesn't linger as a zombie.
    std::thread::spawn(move || {
        let mut child = child;
        let _ = child.wait();
    });
    Ok(())
}

pub fn notify_error(app: &AppHandle, message: &str) {
    let _ = app
        .notification()
        .builder()
        .title("ShareV error")
        .body(message)
        .show();
}

/// Show an informational desktop notification.
pub fn notify_info(app: &AppHandle, title: &str, message: &str) {
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(message)
        .show();
}

/// Show an error notification (used by tray actions that fail).
pub fn capture_error(app: &AppHandle, message: &str) {
    notify_error(app, message);
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
