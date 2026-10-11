//! Compositor / window-manager integration.
//!
//! ShareV needs to enumerate windows and monitors so the user can pick a
//! specific one, and it needs to resolve their pixel geometry for capture and
//! recording. Support is best-effort and only shells out to well-known tools:
//!
//! * Hyprland — `hyprctl`
//! * Sway / wlroots — `swaymsg`, `wlr-randr`
//! * X11 — `wmctrl`, `xrandr`, `xdotool`
//!
//! Every helper returns a plain [`Geometry`] so capture and recording can share
//! the same resolution path.

use serde::Serialize;

use crate::capture::{which, Geometry};
use crate::error::{Error, Result};
use crate::platform::display::{detect, DisplayServer};

/// A window the user can target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// Backend-specific window id (Hyprland address, Sway node id, X11 window id).
    pub id: String,
    pub title: String,
    pub app: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A monitor / output the user can target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub focused: bool,
}

// ---------------------------------------------------------------------------
// Monitors
// ---------------------------------------------------------------------------

/// List all enabled monitors/outputs.
pub async fn list_monitors() -> Result<Vec<MonitorInfo>> {
    if let Ok(monitors) = hyprland_monitors().await {
        return Ok(monitors);
    }
    if let Ok(monitors) = sway_monitors().await {
        return Ok(monitors);
    }
    if let Ok(monitors) = x11_monitors().await {
        return Ok(monitors);
    }
    Err(Error::BackendUnavailable(
        "monitors".into(),
        "could not list monitors (need hyprctl, swaymsg or xrandr)".into(),
    ))
}

/// Geometry of a named monitor, or the focused one when `name` is empty.
pub async fn monitor_geometry(name: &str) -> Result<Geometry> {
    let monitors = list_monitors().await?;
    let wanted = name.trim();
    let chosen = if wanted.is_empty() {
        monitors
            .iter()
            .find(|m| m.focused)
            .or_else(|| monitors.first())
    } else {
        monitors.iter().find(|m| m.name == wanted).or_else(|| {
            // Tolerate a case-insensitive match.
            monitors
                .iter()
                .find(|m| m.name.eq_ignore_ascii_case(wanted))
        })
    };

    chosen.map(|m| Geometry {
        x: m.x,
        y: m.y,
        width: m.width,
        height: m.height,
    }).ok_or_else(|| Error::msg(format!("no monitor named '{wanted}'")))
}

async fn hyprland_monitors() -> Result<Vec<MonitorInfo>> {
    if which("hyprctl").is_none() || detect() != DisplayServer::Wayland {
        return Err(Error::msg("hyprctl not available"));
    }
    let output = tokio::process::Command::new("hyprctl")
        .args(["-j", "monitors"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("hyprctl monitors failed"));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut monitors = Vec::new();
    if let Some(items) = value.as_array() {
        for item in items {
            if item.get("disabled").and_then(|d| d.as_bool()) == Some(true) {
                continue;
            }
            let name = item
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            monitors.push(MonitorInfo {
                name,
                x: item.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: item.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                width: item.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as u32,
                height: item.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as u32,
                focused: item.get("focused").and_then(|v| v.as_bool()) == Some(true),
            });
        }
    }
    Ok(monitors)
}

async fn sway_monitors() -> Result<Vec<MonitorInfo>> {
    if which("swaymsg").is_none() {
        return Err(Error::msg("swaymsg not available"));
    }
    let output = tokio::process::Command::new("swaymsg")
        .args(["-t", "get_outputs", "--raw"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("swaymsg failed"));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut monitors = Vec::new();
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
            let rect = item.get("rect").cloned().unwrap_or(serde_json::Value::Null);
            monitors.push(MonitorInfo {
                name,
                x: rect.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: rect.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                width: rect.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as u32,
                height: rect.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as u32,
                focused: item.get("focused").and_then(|v| v.as_bool()) == Some(true),
            });
        }
    }
    Ok(monitors)
}

async fn x11_monitors() -> Result<Vec<MonitorInfo>> {
    if which("xrandr").is_none() || std::env::var_os("DISPLAY").is_none() {
        return Err(Error::msg("xrandr not available"));
    }
    let output = tokio::process::Command::new("xrandr")
        .arg("--listmonitors")
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("xrandr failed"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let monitors = parse_xrandr_monitors(&text);
    if monitors.is_empty() {
        return Err(Error::msg("xrandr reported no monitors"));
    }
    Ok(monitors)
}

/// Parse the output of `xrandr --listmonitors`.
fn parse_xrandr_monitors(text: &str) -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        // Lines look like: `0: +*eDP-1 1920/344x1080/194+0+0  eDP-1`
        let Some(rest) = line.split_once(':').map(|(_, r)| r.trim()) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() < 3 {
            continue;
        }
        let flags = tokens[0];
        let geometry_token = tokens[1];
        let name = tokens[tokens.len() - 1].to_string();
        let Some((x, y, width, height)) = parse_xrandr_geometry(geometry_token) else {
            continue;
        };
        monitors.push(MonitorInfo {
            name,
            x,
            y,
            width,
            height,
            focused: flags.contains('*'),
        });
    }
    // X11 has no "focused" output; if none is marked primary, focus the first.
    if !monitors.is_empty() && !monitors.iter().any(|m| m.focused) {
        monitors[0].focused = true;
    }
    monitors
}

/// Parse `1920/344x1080/194+0+0` into `(x, y, width, height)`.
fn parse_xrandr_geometry(token: &str) -> Option<(i32, i32, u32, u32)> {
    let (size, offset) = token.split_once('x')?;
    let width: u32 = size.split('/').next()?.trim().parse().ok()?;
    // `offset` is `1080/194+0+0`; strip the millimetre part before the offset.
    let (height_part, rest) = offset.split_once('/')?;
    let height: u32 = height_part.trim().parse().ok()?;
    // `rest` is `194+0+0` (or `194-10+0`). Drop the mm digits.
    let sign_index = rest.find(['+', '-'])?;
    let (x, y) = parse_xy(&rest[sign_index..])?;
    Some((x, y, width, height))
}

/// Parse `+X+Y`, `+X-Y`, `-X+Y` or `-X-Y` into `(x, y)`.
fn parse_xy(text: &str) -> Option<(i32, i32)> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || (bytes[0] != b'+' && bytes[0] != b'-') {
        return None;
    }
    // Find the sign that starts the Y component (searching past index 0).
    let mut split = None;
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if b == b'+' || b == b'-' {
            split = Some(i);
            break;
        }
    }
    let split = split?;
    let x: i32 = text[..split].parse().ok()?;
    let y: i32 = text[split..].parse().ok()?;
    Some((x, y))
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

/// List mapped, user-visible windows.
pub async fn list_windows() -> Result<Vec<WindowInfo>> {
    if let Ok(windows) = hyprland_windows().await {
        return Ok(windows);
    }
    if let Ok(windows) = sway_windows().await {
        return Ok(windows);
    }
    if let Ok(windows) = x11_windows().await {
        return Ok(windows);
    }
    Err(Error::BackendUnavailable(
        "windows".into(),
        "could not list windows (need hyprctl, swaymsg or wmctrl)".into(),
    ))
}

/// Resolve a window id (as returned by [`list_windows`]) to its geometry.
pub async fn window_geometry(id: &str) -> Result<Geometry> {
    let windows = list_windows().await?;
    windows
        .iter()
        .find(|w| w.id == id)
        .map(|w| Geometry {
            x: w.x,
            y: w.y,
            width: w.width,
            height: w.height,
        })
        .ok_or_else(|| Error::msg(format!("window '{id}' was not found")))
}

async fn hyprland_windows() -> Result<Vec<WindowInfo>> {
    if which("hyprctl").is_none() || detect() != DisplayServer::Wayland {
        return Err(Error::msg("hyprctl not available"));
    }
    let output = tokio::process::Command::new("hyprctl")
        .args(["-j", "clients"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("hyprctl clients failed"));
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut windows = Vec::new();
    if let Some(items) = value.as_array() {
        for item in items {
            if item.get("mapped").and_then(|m| m.as_bool()) == Some(false) {
                continue;
            }
            let id = item
                .get("address")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }
            let at = item.get("at").and_then(|v| v.as_array());
            let size = item.get("size").and_then(|v| v.as_array());
            let width = size
                .and_then(|a| a.first())
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u32;
            let height = size
                .and_then(|a| a.get(1))
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as u32;
            if width == 0 || height == 0 {
                continue;
            }
            windows.push(WindowInfo {
                id,
                title: item
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                app: item
                    .get("class")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                x: at.and_then(|a| a.first())
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                y: at.and_then(|a| a.get(1))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32,
                width,
                height,
            });
        }
    }
    Ok(windows)
}

async fn sway_windows() -> Result<Vec<WindowInfo>> {
    if which("swaymsg").is_none() {
        return Err(Error::msg("swaymsg not available"));
    }
    let output = tokio::process::Command::new("swaymsg")
        .args(["-t", "get_tree", "--raw"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("swaymsg failed"));
    }
    let tree: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut windows = Vec::new();
    collect_sway_windows(&tree, &mut windows);
    Ok(windows)
}

fn collect_sway_windows(node: &serde_json::Value, out: &mut Vec<WindowInfo>) {
    // A leaf window node has a `pid` and an app identifier.
    let is_window = node.get("pid").is_some()
        || node.get("app_id").is_some()
        || node.get("window_properties").is_some();
    if is_window {
        let id = node
            .get("id")
            .map(|v| {
                v.as_i64()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| v.to_string())
            })
            .unwrap_or_default();
        let rect = node.get("rect").cloned().unwrap_or(serde_json::Value::Null);
        let width = rect.get("width").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
        let height = rect.get("height").and_then(|v| v.as_i64()).unwrap_or(0) as u32;
        if !id.is_empty() && width > 0 && height > 0 {
            let app = node
                .get("app_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    node.pointer("/window_properties/class")
                        .and_then(|v| v.as_str())
                })
                .unwrap_or("")
                .to_string();
            out.push(WindowInfo {
                id,
                title: node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                app,
                x: rect.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: rect.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                width,
                height,
            });
        }
    }
    for key in ["nodes", "floating_nodes"] {
        if let Some(children) = node.get(key).and_then(|v| v.as_array()) {
            for child in children {
                collect_sway_windows(child, out);
            }
        }
    }
}

async fn x11_windows() -> Result<Vec<WindowInfo>> {
    if which("wmctrl").is_none() || std::env::var_os("DISPLAY").is_none() {
        return Err(Error::msg("wmctrl not available"));
    }
    let output = tokio::process::Command::new("wmctrl")
        .args(["-lG"])
        .output()
        .await?;
    if !output.status.success() {
        return Err(Error::msg("wmctrl failed"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let windows = parse_wmctrl(&text);
    if windows.is_empty() {
        return Err(Error::msg("wmctrl reported no windows"));
    }
    Ok(windows)
}

/// Parse `wmctrl -lG` output.
fn parse_wmctrl(text: &str) -> Vec<WindowInfo> {
    let mut windows = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(id) = fields.next() else { continue };
        // Skip the desktop, then x, y, width, height.
        let _desktop = fields.next();
        let Some(x) = fields.next().and_then(|v| v.parse::<i32>().ok()) else {
            continue;
        };
        let Some(y) = fields.next().and_then(|v| v.parse::<i32>().ok()) else {
            continue;
        };
        let Some(width) = fields.next().and_then(|v| v.parse::<u32>().ok()) else {
            continue;
        };
        let Some(height) = fields.next().and_then(|v| v.parse::<u32>().ok()) else {
            continue;
        };
        // Skip the client machine, then the rest is the title.
        let _host = fields.next();
        let title = fields.collect::<Vec<_>>().join(" ");
        if width == 0 || height == 0 {
            continue;
        }
        windows.push(WindowInfo {
            id: id.to_string(),
            title,
            app: String::new(),
            x,
            y,
            width,
            height,
        });
    }
    windows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_xrandr_monitors() {
        let text = "\
Monitors: 2
 0: +*eDP-1 1920/344x1080/194+0+0  eDP-1
 1: +HDMI-1 2560/600x1440/340+1920+0  HDMI-1
";
        let monitors = parse_xrandr_monitors(text);
        assert_eq!(monitors.len(), 2);
        assert_eq!(monitors[0].name, "eDP-1");
        assert_eq!(
            (monitors[0].x, monitors[0].y, monitors[0].width, monitors[0].height),
            (0, 0, 1920, 1080)
        );
        assert!(monitors[0].focused);
        assert_eq!(monitors[1].name, "HDMI-1");
        assert_eq!(monitors[1].x, 1920);
        assert!(!monitors[1].focused);
    }

    #[test]
    fn parses_negative_xrandr_offsets() {
        let text = " 0: +DP-1 1920/344x1080/194-1920+0  DP-1\n";
        let monitors = parse_xrandr_monitors(text);
        assert_eq!(monitors[0].x, -1920);
        assert_eq!(monitors[0].y, 0);
    }

    #[test]
    fn parses_wmctrl_windows() {
        let text = "\
0x03400007  0 100 200 800 600 host Title With Spaces
0x03a0000a  0 0 0 1920 1080 host Terminal
";
        let windows = parse_wmctrl(text);
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].id, "0x03400007");
        assert_eq!(windows[0].title, "Title With Spaces");
        assert_eq!((windows[0].x, windows[0].y, windows[0].width), (100, 200, 800));
        assert_eq!(windows[1].title, "Terminal");
    }

    #[test]
    fn ignores_zero_sized_windows() {
        let text = "0x1 0 0 0 0 0 host hidden\n";
        assert!(parse_wmctrl(text).is_empty());
    }
}
