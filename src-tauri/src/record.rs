//! Screen recording.
//!
//! Recording is a long-running external process:
//!
//! * on Wayland/wlroots we drive `wf-recorder`;
//! * on X11 we drive `ffmpeg` (`x11grab` + `pulse`);
//! * GIF output is captured to a temporary video and transcoded afterwards.
//!
//! [`start`] spawns the encoder and returns an [`ActiveRecording`] that the
//! application keeps until the user stops it. [`stop`] then asks the process to
//! finish gracefully (SIGINT, so the container is finalized) and [`finalize`]
//! performs any post-processing.

use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Stdio};
use std::time::{Duration, Instant};

use tokio::process::Command;

use crate::capture::{self, CaptureKind, Geometry};
use crate::config::{AudioMode, Config, RecordFormat};
use crate::error::{Error, Result};
use crate::platform::display::{DisplayServer, detect};

/// Backend id reported to the frontend.
pub const BACKEND_WF_RECORDER: &str = "wf-recorder";
pub const BACKEND_FFMPEG: &str = "ffmpeg";

/// Fully specified request to start a recording.
#[derive(Debug, Clone)]
pub struct RecordRequest {
    pub kind: CaptureKind,
    pub geometry: Option<Geometry>,
    pub format: RecordFormat,
    pub fps: u32,
    pub gif_fps: u32,
    pub audio: AudioMode,
    pub system_device: String,
    pub microphone_device: String,
    /// Final output (the GIF, when the format is GIF).
    pub output: PathBuf,
    /// Where the encoder writes (equal to `output` unless transcoding).
    pub record_path: PathBuf,
    pub log_path: PathBuf,
}

/// A recording that is currently in progress.
pub struct ActiveRecording {
    pub child: Child,
    pub output: PathBuf,
    pub record_path: PathBuf,
    pub log_path: PathBuf,
    pub format: RecordFormat,
    pub kind: CaptureKind,
    pub backend: String,
    pub started_at: String,
    pub started: Instant,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub gif_fps: u32,
}

/// True when `wf-recorder` is available and we are on Wayland.
pub fn wf_recorder_available() -> bool {
    matches!(detect(), DisplayServer::Wayland) && capture::which("wf-recorder").is_some()
}

/// True when `ffmpeg` is available (used for X11 and for GIF transcoding).
pub fn ffmpeg_available() -> bool {
    capture::which("ffmpeg").is_some()
}

/// Recording backends, in preference order, with availability.
pub fn backend_infos() -> Vec<crate::capture::BackendInfo> {
    vec![
        crate::capture::BackendInfo {
            id: BACKEND_WF_RECORDER.into(),
            name: "wf-recorder (wlroots)".into(),
            available: wf_recorder_available(),
        },
        crate::capture::BackendInfo {
            id: BACKEND_FFMPEG.into(),
            name: "ffmpeg (x11grab)".into(),
            available: ffmpeg_available(),
        },
    ]
}

pub fn choose_backend() -> Result<&'static str> {
    if wf_recorder_available() {
        Ok(BACKEND_WF_RECORDER)
    } else if ffmpeg_available() {
        Ok(BACKEND_FFMPEG)
    } else {
        Err(Error::BackendUnavailable(
            "recording".into(),
            "install wf-recorder (Wayland) or ffmpeg".into(),
        ))
    }
}

/// Resolve the screen region for a recording request.
pub async fn resolve_geometry(
    kind: CaptureKind,
    backend: &str,
    monitor: &str,
    window_id: Option<&str>,
) -> Result<Option<Geometry>> {
    match kind {
        CaptureKind::Fullscreen => {
            if !monitor.trim().is_empty() {
                return Ok(Some(crate::wm::monitor_geometry(monitor).await?));
            }
            if backend == BACKEND_WF_RECORDER {
                Ok(Some(wayland_fullscreen_geometry().await?))
            } else {
                Ok(Some(x11_screen_geometry().await?))
            }
        }
        CaptureKind::Region | CaptureKind::Scrolling => {
            if backend == BACKEND_WF_RECORDER {
                Ok(Some(crate::capture::wlroots::select_region().await?))
            } else {
                Ok(Some(x11_select_region().await?))
            }
        }
        CaptureKind::Window => {
            if let Some(id) = window_id.filter(|id| !id.trim().is_empty()) {
                return Ok(Some(crate::wm::window_geometry(id).await?));
            }
            if backend == BACKEND_WF_RECORDER {
                Ok(Some(
                    crate::capture::wlroots::active_window_geometry().await?,
                ))
            } else {
                Ok(Some(x11_active_window_geometry().await?))
            }
        }
    }
}

/// Build a fully specified recording request from the configuration.
///
/// Chooses the output path (unique), the temporary capture path (for GIFs) and
/// the encoder log path.
pub fn build_request(
    config: &Config,
    kind: CaptureKind,
    geometry: Option<Geometry>,
) -> RecordRequest {
    let dir = config.resolved_save_dir();
    let file_name = crate::config::build_filename_with_ext(
        &config.filename_template,
        config.record.format.extension(),
    );
    let output = crate::config::unique_path(&dir.join(&file_name));

    // GIF recordings are captured to a temporary video, then transcoded.
    let record_path = if config.record.format == RecordFormat::Gif {
        let stem = output
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "recording".into());
        output.with_file_name(format!("{stem}.rec.mp4"))
    } else {
        output.clone()
    };
    let log_path = output.with_extension("log");

    RecordRequest {
        kind,
        geometry,
        format: config.record.format,
        fps: config.record.fps,
        gif_fps: config.record.gif_fps,
        audio: config.record.audio,
        system_device: config.record.system_device.clone(),
        microphone_device: config.record.microphone_device.clone(),
        output,
        record_path,
        log_path,
    }
}

/// Audio inputs to feed to the encoder, tagged with whether each is system
/// audio so the right "default device" can be chosen when none is configured.
/// The first element is `true` for system audio (a sink monitor).
fn audio_devices(request: &RecordRequest) -> Vec<(bool, String)> {
    let mut devices = Vec::new();
    if request.audio.captures_system() {
        devices.push((true, request.system_device.trim().to_string()));
    }
    if request.audio.captures_microphone() {
        devices.push((false, request.microphone_device.trim().to_string()));
    }
    devices
}

/// Spawn the encoder for `request`.
pub fn start(request: RecordRequest) -> Result<ActiveRecording> {
    let backend = choose_backend()?;

    let log = File::create(&request.log_path)?;
    let mut command = if backend == BACKEND_WF_RECORDER {
        wf_recorder_command(&request)
    } else {
        ffmpeg_command(&request)?
    };

    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(Error::Io)?;

    Ok(ActiveRecording {
        child,
        output: request.output,
        record_path: request.record_path,
        log_path: request.log_path,
        format: request.format,
        kind: request.kind,
        backend: backend.to_string(),
        started_at: chrono::Local::now().to_rfc3339(),
        started: Instant::now(),
        width: request.geometry.map(|g| g.width).unwrap_or(0),
        height: request.geometry.map(|g| g.height).unwrap_or(0),
        fps: request.fps,
        gif_fps: request.gif_fps,
    })
}

fn wf_recorder_command(request: &RecordRequest) -> StdCommand {
    let mut command = StdCommand::new("wf-recorder");

    match request.format {
        RecordFormat::Webm => {
            command.arg("-c").arg("libvpx-vp9");
        }
        // MP4 and the GIF intermediate both use H.264.
        RecordFormat::Mp4 | RecordFormat::Gif => {
            command.arg("-c").arg("libx264");
        }
    }

    if request.fps > 0 {
        command.arg("-r").arg(request.fps.to_string());
    }
    if let Some(geometry) = request.geometry {
        command.arg("-g").arg(geometry.to_grim());
    }
    for (system, device) in audio_devices(request) {
        if device.is_empty() {
            if system {
                // Bare `-a` records the default sink's monitor.
                command.arg("-a");
            } else {
                // The default input source is the microphone.
                command.arg("-a@DEFAULT_SOURCE@");
            }
        } else {
            command.arg(format!("-a{device}"));
        }
    }

    command.arg("-f").arg(&request.record_path);
    command
}

fn ffmpeg_command(request: &RecordRequest) -> Result<StdCommand> {
    let geometry = request
        .geometry
        .ok_or_else(|| Error::msg("X11 recording needs a screen geometry"))?;

    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".into());
    let mut command = StdCommand::new("ffmpeg");
    command
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .arg("-f")
        .arg("x11grab")
        .arg("-framerate")
        .arg(request.fps.max(1).to_string())
        .arg("-video_size")
        .arg(format!("{}x{}", geometry.width, geometry.height))
        .arg("-i")
        .arg(format!("{display}+{},{}", geometry.x, geometry.y));

    let devices = audio_devices(request);
    for (system, device) in &devices {
        command.arg("-f").arg("pulse");
        command.arg("-i").arg(device_or_default(*system, device));
    }

    match request.format {
        RecordFormat::Mp4 | RecordFormat::Gif => {
            command
                .args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "23"])
                .args(["-pix_fmt", "yuv420p"]);
        }
        RecordFormat::Webm => {
            command.args(["-c:v", "libvpx-vp9", "-crf", "32", "-b:v", "0"]);
        }
    }

    match devices.len() {
        0 => {}
        1 => {
            command
                .args(["-c:a", "aac"])
                .args(["-map", "0:v"])
                .args(["-map", "1:a"]);
        }
        _ => {
            command
                .arg("-filter_complex")
                .arg("[1:a][2:a]amix=inputs=2[a]")
                .args(["-map", "0:v"])
                .args(["-map", "[a]"])
                .args(["-c:a", "aac"]);
        }
    }

    command.arg(&request.record_path);
    Ok(command)
}

fn device_or_default(system: bool, device: &str) -> String {
    if !device.is_empty() {
        return device.to_string();
    }
    // PulseAudio special names: the default sink's monitor for system audio,
    // and the default source (microphone) otherwise.
    if system {
        "@DEFAULT_MONITOR@".into()
    } else {
        "@DEFAULT_SOURCE@".into()
    }
}

/// Ask the encoder to stop and wait for it to finish writing the file.
pub fn stop(recording: &mut ActiveRecording) -> Result<()> {
    signal_interrupt(recording.child.id());

    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match recording.child.try_wait()? {
            Some(_) => return Ok(()),
            None if Instant::now() >= deadline => {
                // Refused to exit — force it.
                let _ = recording.child.kill();
                let _ = recording.child.wait();
                return Ok(());
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// Post-process a finished recording (GIF transcoding) and return the final path.
pub async fn finalize(recording: &ActiveRecording) -> Result<()> {
    if recording.format == RecordFormat::Gif {
        convert_to_gif(
            &recording.record_path,
            &recording.output,
            recording.gif_fps.max(1),
        )
        .await?;
        let _ = std::fs::remove_file(&recording.record_path);
    }
    let _ = std::fs::remove_file(&recording.log_path);
    Ok(())
}

/// Read the encoder log; used to explain a failed recording.
pub fn log_tail(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let tail: String = text
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    if tail.trim().is_empty() {
        None
    } else {
        Some(tail)
    }
}

async fn convert_to_gif(input: &Path, output: &Path, fps: u32) -> Result<()> {
    let filter = format!(
        "fps={fps},scale=iw:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse"
    );
    let status = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(input)
        .arg("-vf")
        .arg(filter)
        .args(["-loop", "0"])
        .arg(output)
        .status()
        .await
        .map_err(Error::Io)?;
    if !status.success() {
        return Err(Error::msg("could not convert the recording to GIF"));
    }
    Ok(())
}

fn signal_interrupt(pid: u32) {
    // SIGINT lets wf-recorder and ffmpeg finalize their containers cleanly.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGINT);
    }
}

// ---------------------------------------------------------------------------
// Wayland geometry helpers
// ---------------------------------------------------------------------------

/// Geometry of the focused output, so a fullscreen recording on Wayland maps to
/// a single wlroots output (wf-recorder ≥ 0.5 requires one).
async fn wayland_fullscreen_geometry() -> Result<Geometry> {
    if let Some(geometry) = hyprland_focused_output().await {
        return Ok(geometry);
    }
    if let Some(geometry) = sway_focused_output().await {
        return Ok(geometry);
    }
    if let Some(geometry) = wlr_randr_single_output().await {
        return Ok(geometry);
    }
    Err(Error::BackendUnavailable(
        "wlroots".into(),
        "could not determine the focused output; run Hyprland (hyprctl) or Sway (swaymsg)".into(),
    ))
}

fn geometry_from_rect(value: &serde_json::Value) -> Option<Geometry> {
    let x = value.get("x")?.as_i64()? as i32;
    let y = value.get("y")?.as_i64()? as i32;
    let width = value.get("width")?.as_i64()? as u32;
    let height = value.get("height")?.as_i64()? as u32;
    if width == 0 || height == 0 {
        return None;
    }
    Some(Geometry {
        x,
        y,
        width,
        height,
    })
}

async fn hyprland_focused_output() -> Option<Geometry> {
    capture::which("hyprctl")?;
    let output = Command::new("hyprctl")
        .args(["-j", "monitors"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let monitors: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let monitors = monitors.as_array()?;
    let focused = monitors
        .iter()
        .find(|m| m.get("focused").and_then(|f| f.as_bool()) == Some(true))
        .or_else(|| monitors.first())?;
    geometry_from_rect(focused)
}

async fn sway_focused_output() -> Option<Geometry> {
    capture::which("swaymsg")?;
    let output = Command::new("swaymsg")
        .args(["-t", "get_outputs", "--raw"])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let outputs: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let outputs = outputs.as_array()?;
    let focused = outputs
        .iter()
        .find(|o| o.get("focused").and_then(|f| f.as_bool()) == Some(true))
        .or_else(|| outputs.first())?;
    let rect = focused.get("rect")?;
    geometry_from_rect(rect)
}

/// Fallback used when neither Hyprland nor Sway is available: only succeed when
/// a single output is enabled, using its current mode.
async fn wlr_randr_single_output() -> Option<Geometry> {
    capture::which("wlr-randr")?;
    let output = Command::new("wlr-randr")
        .arg("--json")
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let outputs: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let enabled: Vec<&serde_json::Value> = outputs
        .as_array()?
        .iter()
        .filter(|o| o.get("enabled").and_then(|e| e.as_bool()) == Some(true))
        .collect();
    if enabled.len() != 1 {
        return None;
    }
    let modes = enabled[0].get("modes")?.as_array()?;
    let current = modes
        .iter()
        .find(|m| m.get("current").and_then(|c| c.as_bool()) == Some(true))?;
    let width = current.get("width")?.as_i64()? as u32;
    let height = current.get("height")?.as_i64()? as u32;
    if width == 0 || height == 0 {
        return None;
    }
    Some(Geometry {
        x: 0,
        y: 0,
        width,
        height,
    })
}

// ---------------------------------------------------------------------------
// X11 geometry helpers
// ---------------------------------------------------------------------------

async fn x11_screen_geometry() -> Result<Geometry> {
    if capture::which("xdpyinfo").is_none() {
        return Err(Error::msg("fullscreen recording on X11 needs `xdpyinfo`"));
    }
    let output = Command::new("xdpyinfo").output().await.map_err(Error::Io)?;
    if !output.status.success() {
        return Err(Error::msg("xdpyinfo failed"));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("dimensions:") else {
            continue;
        };
        let Some(size) = rest.split_whitespace().next() else {
            continue;
        };
        let Some((w, h)) = size.split_once('x') else {
            continue;
        };
        let geometry = Geometry {
            x: 0,
            y: 0,
            width: w.trim().parse().unwrap_or(0),
            height: h.trim().parse().unwrap_or(0),
        };
        if geometry.width > 0 && geometry.height > 0 {
            return Ok(geometry);
        }
    }
    Err(Error::msg("could not determine the screen size"))
}

async fn x11_select_region() -> Result<Geometry> {
    if capture::which("slop").is_none() {
        return Err(Error::BackendUnavailable(
            "x11".into(),
            "region recording needs `slop` (install it)".into(),
        ));
    }
    let output = Command::new("slop")
        .args(["-f", "%x,%y %wx%h"])
        .output()
        .await
        .map_err(Error::Io)?;
    if !output.status.success() {
        return Err(Error::Cancelled);
    }
    Geometry::parse(&String::from_utf8_lossy(&output.stdout))
}

async fn x11_active_window_geometry() -> Result<Geometry> {
    if capture::which("xdotool").is_none() {
        return Err(Error::BackendUnavailable(
            "x11".into(),
            "window recording needs `xdotool`".into(),
        ));
    }
    let output = Command::new("xdotool")
        .args(["getactivewindow", "getwindowgeometry", "--shell"])
        .output()
        .await
        .map_err(Error::Io)?;
    if !output.status.success() {
        return Err(Error::msg("xdotool could not find the active window"));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut geometry = Geometry {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        match key.trim() {
            "X" => geometry.x = value.trim().parse().unwrap_or(0),
            "Y" => geometry.y = value.trim().parse().unwrap_or(0),
            "WIDTH" => geometry.width = value.trim().parse().unwrap_or(0),
            "HEIGHT" => geometry.height = value.trim().parse().unwrap_or(0),
            _ => {}
        }
    }
    if geometry.width == 0 || geometry.height == 0 {
        return Err(Error::msg("window has zero size"));
    }
    Ok(geometry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(format: RecordFormat, audio: AudioMode) -> RecordRequest {
        RecordRequest {
            kind: CaptureKind::Region,
            geometry: Some(Geometry {
                x: 10,
                y: 20,
                width: 300,
                height: 200,
            }),
            format,
            fps: 30,
            gif_fps: 15,
            audio,
            system_device: "monitor".into(),
            microphone_device: "mic".into(),
            output: PathBuf::from("/tmp/out.mp4"),
            record_path: PathBuf::from("/tmp/out.mp4"),
            log_path: PathBuf::from("/tmp/out.log"),
        }
    }

    #[test]
    fn builds_wf_recorder_arguments() {
        let command = wf_recorder_command(&request(RecordFormat::Webm, AudioMode::None));
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        // codec, framerate and geometry are present.
        assert!(args.windows(2).any(|w| w == ["-c", "libvpx-vp9"]));
        assert!(args.windows(2).any(|w| w == ["-r", "30"]));
        assert!(args.windows(2).any(|w| w == ["-g", "10,20 300x200"]));
        assert!(args.windows(2).any(|w| w[0] == "-f"));
        assert!(!args.iter().any(|a| a.starts_with("-a")));
    }

    #[test]
    fn wf_recorder_adds_audio_devices() {
        let command = wf_recorder_command(&request(RecordFormat::Mp4, AudioMode::Both));
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|a| a == "-amonitor"));
        assert!(args.iter().any(|a| a == "-amic"));
    }

    #[test]
    fn wf_recorder_gif_uses_h264_intermediate() {
        let command = wf_recorder_command(&request(RecordFormat::Gif, AudioMode::None));
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|w| w == ["-c", "libx264"]));
    }

    #[test]
    fn builds_ffmpeg_arguments_with_mixed_audio() {
        let command = ffmpeg_command(&request(RecordFormat::Mp4, AudioMode::Both)).unwrap();
        let args: Vec<_> = command
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|w| w[0] == "-f" && w[1] == "x11grab"));
        assert!(
            args.windows(2)
                .any(|w| w[0] == "-video_size" && w[1] == "300x200")
        );
        // two pulse inputs and an amix filter
        assert_eq!(args.iter().filter(|a| *a == "pulse").count(), 2);
        assert!(args.iter().any(|a| a.contains("amix")));
    }

    #[test]
    fn ffmpeg_requires_geometry() {
        let mut req = request(RecordFormat::Mp4, AudioMode::None);
        req.geometry = None;
        assert!(ffmpeg_command(&req).is_err());
    }
}
