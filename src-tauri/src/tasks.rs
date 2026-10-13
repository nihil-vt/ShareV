//! Post-capture task execution.
//!
//! A capture is grabbed and encoded by [`crate::pipeline::grab`], then this
//! module runs the ordered [`Task`] list configured under `[afterCapture]`.
//! Tasks that need a running app (notifications, pinned windows) are no-ops
//! when no [`AppHandle`] is available (the headless CLI).

use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::commands::notify_error;
use crate::config::{Config, Task, build_filename, unique_path};
use crate::error::Result;
use crate::pipeline::{CaptureDraft, CaptureResult};
use crate::{clipboard, history, upload};

/// Execute the configured after-capture tasks and build the capture result.
pub async fn run(
    app: Option<&AppHandle>,
    config: &Config,
    draft: CaptureDraft,
) -> Result<CaptureResult> {
    let id = crate::id::new_id();
    let created_at = chrono::Local::now().to_rfc3339();
    let mut width = draft.image.width();
    let mut height = draft.image.height();
    let format = draft.format;
    let mut byte_count = draft.encoded.len() as u64;

    let mut path: Option<std::path::PathBuf> = None;
    let mut file_name = String::new();
    let mut copied = false;
    let mut url: Option<String> = None;
    let open_editor = false;

    for task in config.after_capture_tasks() {
        match task {
            Task::Save => {
                let dir = config.resolved_save_dir();
                let name = build_filename(&config.filename_template, format);
                let target = unique_path(&dir.join(&name));
                std::fs::write(&target, &draft.encoded)?;
                file_name = target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or(name);
                path = Some(target.clone());

                // Apply image effects after saving if any are configured. Re-encode
                // through the pipeline so the output format (and JPEG flattening)
                // stays consistent, then refresh the recorded dimensions/bytes.
                if !config.effects.effects.is_empty() {
                    match image::open(&target) {
                        Ok(img) => {
                            let effected = crate::effects::apply_effects(&img, &config.effects);
                            match crate::pipeline::encode(
                                &effected.to_rgba8(),
                                format,
                                config.jpeg_quality,
                            ) {
                                Ok(bytes) => {
                                    std::fs::write(&target, &bytes)?;
                                    width = effected.width();
                                    height = effected.height();
                                    byte_count = bytes.len() as u64;
                                }
                                Err(err) => log::warn!("could not encode image effects: {err}"),
                            }
                        }
                        Err(err) => log::warn!("could not apply image effects: {err}"),
                    }
                }
            }

            Task::History => {
                if let Some(target) = &path {
                    let entry = history::HistoryEntry {
                        id: id.clone(),
                        path: target.to_string_lossy().into_owned(),
                        file_name: file_name.clone(),
                        kind: draft.kind.as_str().to_string(),
                        width,
                        height,
                        bytes: byte_count,
                        backend: draft.backend.clone(),
                        created_at: created_at.clone(),
                        url: None,
                    };
                    history::append(entry, config.max_history)?;
                }
            }

            Task::CopyImage => {
                let result = match &path {
                    Some(target) => clipboard::copy_image(target, format),
                    None => clipboard::copy_bytes(&draft.encoded, format.mime()),
                };
                match result {
                    Ok(()) => copied = true,
                    Err(err) => log::warn!("clipboard copy failed: {err}"),
                }
            }

            Task::Editor => {
                if let (Some(app), Some(target)) = (app, &path)
                    && let Err(err) = crate::commands::open_editor(
                        app.clone(),
                        target.to_string_lossy().into_owned(),
                    )
                {
                    log::warn!("could not open editor: {err}");
                }
            }

            Task::Open => {
                if let Some(target) = &path
                    && let Err(err) = crate::commands::open_with(&target.to_string_lossy())
                {
                    log::warn!("could not open capture: {err}");
                }
            }

            Task::Upload => {
                if let Some(target) = &path {
                    match upload::upload(config, target).await {
                        Ok(remote) => {
                            let link = config.upload.copy_format.format(&remote);
                            if clipboard::copy_text(&link).is_err() {
                                log::warn!("could not copy the upload link to the clipboard");
                            }
                            if let Some(mut entry) =
                                history::find_by_path(&target.to_string_lossy())
                            {
                                entry.url = Some(remote.clone());
                                if let Err(err) = history::replace(entry) {
                                    log::warn!("could not record upload url: {err}");
                                }
                            }
                            url = Some(remote);
                        }
                        Err(err) => log::warn!("upload task failed: {err}"),
                    }
                }
            }

            Task::Pin => {
                if let (Some(app), Some(target)) = (app, &path)
                    && let Err(err) = crate::pin::pin(
                        app,
                        &target.to_string_lossy(),
                        width,
                        height,
                        draft.geometry,
                    )
                {
                    log::warn!("could not pin capture: {err}");
                }
            }

            Task::Command => {
                if let Some(target) = &path
                    && let Err(err) = run_command(config, target).await
                {
                    log::warn!("after-capture command failed: {err}");
                }
            }

            Task::Notify => {
                notify(app, &file_name, width, height, url.as_deref());
            }

            Task::CopyLink => {}

            // Handled before the loop in `after_upload` — the shortened URL
            // must be what gets copied/opened/notified.
            Task::ShortenUrl => {}

            // The frontend shows the quick task menu when this task is
            // enabled; nothing to do in the backend chain.
            Task::QuickMenu => {}

            Task::Ocr => {
                if !config.features.ocr {
                    log::debug!("skipping OCR task: the OCR feature is disabled");
                    continue;
                }
                if let Some(target) = &path {
                    let lang = config.ocr_language.clone();
                    match crate::ocr::recognize(&target.to_string_lossy(), &lang).await {
                        Ok(text) => {
                            if !text.is_empty()
                                && let Err(err) = crate::clipboard::copy_text(&text)
                            {
                                log::warn!("could not copy OCR text to clipboard: {err}");
                            }
                        }
                        Err(err) => {
                            log::warn!("OCR failed: {err}");
                            if let Some(app) = app {
                                notify_error(app, &format!("OCR failed: {err}"));
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(CaptureResult {
        id,
        path: path
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        file_name,
        kind: draft.kind.as_str().to_string(),
        width,
        height,
        bytes: byte_count,
        backend: draft.backend,
        created_at,
        copied,
        url,
        open_editor,
    })
}

/// Run the configured post-upload tasks for a freshly uploaded `url`.
///
/// Returns the formatted link and whether it was copied to the clipboard.
pub async fn after_upload(app: Option<&AppHandle>, config: &Config, url: &str) -> (String, bool) {
    // Shorten first: the copy/open tasks below must see the short link.
    let mut target = url.to_string();
    if config
        .after_upload_tasks()
        .contains(&crate::config::Task::ShortenUrl)
    {
        let provider = crate::shorten::ShortenProvider::parse(&config.shorten.provider)
            .unwrap_or(crate::shorten::ShortenProvider::Isgd);
        let creds = crate::shorten::ShortenConfig::for_provider(config, provider);
        match crate::shorten::shorten(&target, provider, &creds).await {
            Ok(shortened) => target = shortened,
            Err(err) => log::warn!("url shortening failed: {err}"),
        }
    }

    let link = config.upload.copy_format.format(&target);
    let mut copied = false;

    for task in config.after_upload_tasks() {
        match task {
            Task::CopyLink => {
                if let Err(err) = crate::clipboard::copy_text(&link) {
                    log::warn!("could not copy the upload link: {err}");
                } else {
                    copied = true;
                }
            }
            Task::Notify => {
                let body = format!("Uploaded · {target}");
                if let Some(app) = app {
                    let _ = app
                        .notification()
                        .builder()
                        .title("ShareV")
                        .body(body)
                        .show();
                } else {
                    let _ = std::process::Command::new("notify-send")
                        .arg("ShareV")
                        .arg(&body)
                        .status();
                }
            }
            Task::Open => {
                if let Err(err) = crate::commands::open_with(&target) {
                    log::warn!("could not open upload url: {err}");
                }
            }
            _ => {}
        }
    }

    (link, copied)
}

/// Run the configured post-capture command on `path`.
async fn run_command(config: &Config, path: &Path) -> Result<()> {
    let template = config.after_capture.command.trim();
    if template.is_empty() {
        return Ok(());
    }

    let file = path.to_string_lossy().into_owned();
    let command = if template.contains("{file}") {
        template.replace("{file}", &upload::shell_quote(&file))
    } else {
        format!("{template} {}", upload::shell_quote(&file))
    };

    let status = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .env("SHAREV_FILE", &file)
        .status()
        .await?;

    if !status.success() {
        return Err(crate::error::Error::msg(format!(
            "command exited with {status}"
        )));
    }
    Ok(())
}

/// Show a desktop notification, preferring the in-app plugin.
fn notify(app: Option<&AppHandle>, file_name: &str, width: u32, height: u32, url: Option<&str>) {
    let file_name = if file_name.is_empty() {
        "capture"
    } else {
        file_name
    };
    let body = match url {
        Some(url) => format!("{file_name} · {url}"),
        None => format!("Saved {file_name} ({width}x{height})"),
    };

    if let Some(app) = app {
        let _ = app
            .notification()
            .builder()
            .title("ShareV")
            .body(body)
            .show();
        return;
    }

    // Headless fallback.
    let _ = std::process::Command::new("notify-send")
        .arg("ShareV")
        .arg(&body)
        .status();
}
