use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

use super::backend::{decode_png, CaptureBackend, CaptureRequest, CapturedImage, Geometry};
use crate::capture::which;
use crate::error::{Error, Result};

/// Wayland capture using `grim` for the pixels and `slurp` for region selection.
/// Works on wlroots-based compositors (Hyprland, Sway, etc.).
pub struct WlrootsBackend;

fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

#[async_trait]
impl CaptureBackend for WlrootsBackend {
    fn id(&self) -> &'static str {
        "wlroots"
    }

    fn name(&self) -> &'static str {
        "wlroots (grim + slurp)"
    }

    fn is_available(&self) -> bool {
        is_wayland() && which("grim").is_some()
    }

    async fn capture(&self, request: CaptureRequest) -> Result<CapturedImage> {
        let geometry = match request {
            CaptureRequest::Fullscreen => None,
            CaptureRequest::Region { geometry } => Some(match geometry {
                Some(g) => g,
                None => select_region().await?,
            }),
            CaptureRequest::Window { .. } => Some(active_window_geometry().await?),
        };

        let mut cmd = Command::new("grim");
        if let Some(g) = geometry {
            cmd.arg("-g").arg(g.to_grim());
        }
        cmd.arg("-")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().await.map_err(Error::Io)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                return Err(Error::Cancelled);
            }
            return Err(Error::BackendUnavailable("wlroots".into(), stderr));
        }
        if output.stdout.is_empty() {
            return Err(Error::EmptyCapture);
        }

        Ok(CapturedImage {
            image: decode_png(&output.stdout)?,
            backend: self.id().to_string(),
        })
    }
}

pub async fn select_region() -> Result<Geometry> {
    let output = Command::new("slurp").output().await.map_err(Error::Io)?;
    if !output.status.success() {
        // slurp exits non-zero when the user cancels the selection.
        return Err(Error::Cancelled);
    }
    Geometry::parse(&String::from_utf8_lossy(&output.stdout))
}

pub async fn active_window_geometry() -> Result<Geometry> {
    if let Ok(g) = hyprland_active_window().await {
        return Ok(g);
    }
    if let Ok(g) = sway_active_window().await {
        return Ok(g);
    }
    Err(Error::BackendUnavailable(
        "wlroots".into(),
        "active-window capture needs Hyprland (hyprctl) or Sway (swaymsg)".into(),
    ))
}

async fn hyprland_active_window() -> Result<Geometry> {
    if which("hyprctl").is_none() {
        return Err(Error::msg("hyprctl not found"));
    }
    let output = Command::new("hyprctl")
        .args(["-j", "activewindow"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("hyprctl failed"));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    if value.get("mapped").and_then(|m| m.as_bool()) == Some(false) {
        return Err(Error::msg("no active window"));
    }
    let at = value
        .get("at")
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::msg("missing 'at'"))?;
    let size = value
        .get("size")
        .and_then(|a| a.as_array())
        .ok_or_else(|| Error::msg("missing 'size'"))?;

    let x = at.first().and_then(|n| n.as_i64()).unwrap_or(0) as i32;
    let y = at.get(1).and_then(|n| n.as_i64()).unwrap_or(0) as i32;
    let width = size.first().and_then(|n| n.as_i64()).unwrap_or(0) as u32;
    let height = size.get(1).and_then(|n| n.as_i64()).unwrap_or(0) as u32;
    if width == 0 || height == 0 {
        return Err(Error::msg("window has zero size"));
    }
    Ok(Geometry {
        x,
        y,
        width,
        height,
    })
}

async fn sway_active_window() -> Result<Geometry> {
    if which("swaymsg").is_none() {
        return Err(Error::msg("swaymsg not found"));
    }
    let output = Command::new("swaymsg")
        .args(["-t", "get_tree", "--raw"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("swaymsg failed"));
    }
    let tree: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let node = find_focused(&tree).ok_or_else(|| Error::msg("no focused window"))?;
    let rect = node.get("rect").ok_or_else(|| Error::msg("missing rect"))?;
    let x = rect.get("x").and_then(|n| n.as_i64()).unwrap_or(0) as i32;
    let y = rect.get("y").and_then(|n| n.as_i64()).unwrap_or(0) as i32;
    let width = rect.get("width").and_then(|n| n.as_i64()).unwrap_or(0) as u32;
    let height = rect.get("height").and_then(|n| n.as_i64()).unwrap_or(0) as u32;
    if width == 0 || height == 0 {
        return Err(Error::msg("window has zero size"));
    }
    Ok(Geometry {
        x,
        y,
        width,
        height,
    })
}

fn find_focused(node: &serde_json::Value) -> Option<&serde_json::Value> {
    if node.get("focused").and_then(|f| f.as_bool()) == Some(true) {
        return Some(node);
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(|n| n.as_array()) {
            for child in children {
                if let Some(found) = find_focused(child) {
                    return Some(found);
                }
            }
        }
    }
    None
}
