//! Post-capture task execution.
//!
//! A capture is grabbed and encoded by [`crate::pipeline::grab`], then this
//! module runs the ordered [`Task`] list configured under `[afterCapture]`.
//! Tasks that need a running app (notifications, pinned windows) are no-ops
//! when no [`AppHandle`] is available (the headless CLI).

use std::path::Path;

use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

use crate::config::{build_filename, unique_path, Config, Task};
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
    let width = draft.image.width();
    let height = draft.image.height();
    let format = draft.format;

    let mut path: Option<std::path::PathBuf> = None;
    let mut file_name = String::new();
    let mut copied = false;
    let mut url: Option<String> = None;
    let mut open_editor = false;

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
                path = Some(target);
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
                        bytes: draft.encoded.len() as u64,
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

            Task::Editor => open_editor = true,

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
                            if let Some(mut entry) = history::find_by_path(&target.to_string_lossy())
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
                    && let Err(err) =
                        crate::pin::pin(app, &target.to_string_lossy(), width, height, draft.geometry)
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

            // Only meaningful after an upload; ignore here.
            Task::CopyLink => {}
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
        bytes: draft.encoded.len() as u64,
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
    let link = config.upload.copy_format.format(url);
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
                let body = format!("Uploaded · {url}");
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
                if let Err(err) = crate::commands::open_with(url) {
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
fn notify(
    app: Option<&AppHandle>,
    file_name: &str,
    width: u32,
    height: u32,
    url: Option<&str>,
) {
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
