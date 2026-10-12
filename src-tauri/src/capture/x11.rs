use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::backend::{CaptureBackend, CaptureRequest, CapturedImage, decode_png};
use crate::capture::which;
use crate::error::{Error, Result};

/// X11 capture using ImageMagick's `import`.
///
/// `import` can grab the root window (fullscreen), run its own interactive
/// region selector, or grab a specific window id found via `xdotool`.
pub struct X11Backend;

fn is_x11() -> bool {
    std::env::var_os("DISPLAY").is_some()
}

#[async_trait]
impl CaptureBackend for X11Backend {
    fn id(&self) -> &'static str {
        "x11"
    }

    fn name(&self) -> &'static str {
        "X11 (ImageMagick import)"
    }

    fn is_available(&self) -> bool {
        is_x11() && which("import").is_some()
    }

    async fn capture(&self, request: CaptureRequest) -> Result<CapturedImage> {
        let mut cmd = Command::new("import");
        let mut geometry = None;
        // Only the bare region request runs `import`'s own interactive selector;
        // for every other case a non-zero exit is a real error, not a cancel.
        let interactive = matches!(request, CaptureRequest::Region { geometry: None });
        match request {
            CaptureRequest::Fullscreen { monitor } => {
                if let Some(name) = monitor.filter(|n| !n.trim().is_empty()) {
                    let g = crate::wm::monitor_geometry(&name).await?;
                    cmd.arg("-crop")
                        .arg(format!("{}x{}+{}+{}", g.width, g.height, g.x, g.y));
                    geometry = Some(g);
                }
                cmd.args(["-window", "root"]);
            }
            // With no geometry, `import` opens its own interactive selector.
            CaptureRequest::Region { geometry: None } => {}
            CaptureRequest::Region { geometry: Some(g) } => {
                cmd.arg("-crop")
                    .arg(format!("{}x{}+{}+{}", g.width, g.height, g.x, g.y));
                cmd.args(["-window", "root"]);
                geometry = Some(g);
            }
            CaptureRequest::Window { id } => {
                let id = match id.filter(|i| !i.trim().is_empty()) {
                    Some(id) => id,
                    None => active_window_id().await?,
                };
                cmd.args(["-window", &id]);
            }
        }
        cmd.arg("png:-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await.map_err(Error::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if interactive && stderr.is_empty() {
                return Err(Error::Cancelled);
            }
            let message = if stderr.is_empty() {
                format!("import exited with status {}", output.status)
            } else {
                stderr
            };
            return Err(Error::BackendUnavailable("x11".into(), message));
        }
        if output.stdout.is_empty() {
            return Err(Error::EmptyCapture);
        }

        Ok(CapturedImage {
            image: decode_png(&output.stdout)?,
            backend: self.id().to_string(),
            geometry,
        })
    }
}

async fn active_window_id() -> Result<String> {
    if which("xdotool").is_none() {
        return Err(Error::BackendUnavailable(
            "x11".into(),
            "window capture needs `xdotool` (install it) ".into(),
        ));
    }
    let output = Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("xdotool could not find the active window"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
