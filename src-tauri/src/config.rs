use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::Result;
use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
}

impl ImageFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Webp => "webp",
        }
    }

    pub fn mime(self) -> &'static str {
        match self {
            ImageFormat::Png => "image/png",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Webp => "image/webp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    pub enabled: bool,
    /// Shortcut for interactive region capture, e.g. "PrintScreen".
    pub region: String,
    /// Shortcut for fullscreen capture. Empty disables it.
    pub fullscreen: String,
    /// Shortcut for active-window capture. Empty disables it.
    pub window: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            enabled: true,
            region: "PrintScreen".into(),
            fullscreen: "Ctrl+PrintScreen".into(),
            window: "Alt+PrintScreen".into(),
        }
    }
}

/// Which destination uploaded images are sent to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploaderKind {
    None,
    Imgur,
    Http,
    Command,
}

impl UploaderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UploaderKind::None => "none",
            UploaderKind::Imgur => "imgur",
            UploaderKind::Http => "http",
            UploaderKind::Command => "command",
        }
    }
}

impl Default for UploaderKind {
    fn default() -> Self {
        UploaderKind::None
    }
}

/// How the returned link is formatted before being copied to the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyFormat {
    Url,
    Markdown,
    Html,
}

impl CopyFormat {
    pub fn format(self, url: &str) -> String {
        match self {
            CopyFormat::Url => url.to_string(),
            CopyFormat::Markdown => format!("![](<{url}>)"),
            CopyFormat::Html => format!("<img src=\"{url}\" />"),
        }
    }
}

impl Default for CopyFormat {
    fn default() -> Self {
        CopyFormat::Url
    }
}

/// Upload / share configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UploadConfig {
    /// Active destination: "none", "imgur", "http" or "command".
    pub uploader: UploaderKind,
    /// Format copied to the clipboard after a successful upload.
    pub copy_format: CopyFormat,
    /// Imgur application Client-ID (free, anonymous).
    pub imgur_client_id: String,
    /// Generic HTTP endpoint that accepts a multipart file upload.
    pub http_url: String,
    /// Multipart form field name for the file (default "file").
    pub http_field: String,
    /// Extra request headers, one "Name: Value" per line.
    pub http_headers: String,
    /// Dotted JSON path to the URL in the response (empty = raw body).
    pub http_response_path: String,
    /// Shell command used by the "command" uploader.
    pub command: String,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            uploader: UploaderKind::None,
            copy_format: CopyFormat::Url,
            imgur_client_id: String::new(),
            http_url: String::new(),
            http_field: "file".into(),
            http_headers: String::new(),
            http_response_path: String::new(),
            command: String::new(),
        }
    }
}

/// Container / codec for screen recordings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordFormat {
    Mp4,
    Webm,
    Gif,
}

impl RecordFormat {
    pub fn extension(self) -> &'static str {
        match self {
            RecordFormat::Mp4 => "mp4",
            RecordFormat::Webm => "webm",
            RecordFormat::Gif => "gif",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RecordFormat::Mp4 => "mp4",
            RecordFormat::Webm => "webm",
            RecordFormat::Gif => "gif",
        }
    }
}

impl Default for RecordFormat {
    fn default() -> Self {
        RecordFormat::Mp4
    }
}

/// Which audio sources a recording captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioMode {
    None,
    System,
    Microphone,
    Both,
}

impl AudioMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AudioMode::None => "none",
            AudioMode::System => "system",
            AudioMode::Microphone => "microphone",
            AudioMode::Both => "both",
        }
    }

    pub fn captures_system(self) -> bool {
        matches!(self, AudioMode::System | AudioMode::Both)
    }

    pub fn captures_microphone(self) -> bool {
        matches!(self, AudioMode::Microphone | AudioMode::Both)
    }
}

impl Default for AudioMode {
    fn default() -> Self {
        AudioMode::None
    }
}

/// Screen recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RecordConfig {
    /// Output container/codec: "mp4", "webm" or "gif".
    pub format: RecordFormat,
    /// Capture framerate.
    pub fps: u32,
    /// Framerate used when converting a recording to GIF.
    pub gif_fps: u32,
    /// Audio sources: "none", "system", "microphone" or "both".
    pub audio: AudioMode,
    /// PulseAudio/PipeWire monitor device. Empty = the default sink monitor.
    pub system_device: String,
    /// PulseAudio/PipeWire source device. Empty = the default source.
    pub microphone_device: String,
}

impl Default for RecordConfig {
    fn default() -> Self {
        Self {
            format: RecordFormat::Mp4,
            fps: 30,
            gif_fps: 15,
            audio: AudioMode::None,
            system_device: String::new(),
            microphone_device: String::new(),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    /// Directory captures are written to.
    pub save_directory: String,
    /// File format for saved captures.
    pub image_format: ImageFormat,
    /// JPEG/WebP quality (1-100).
    pub jpeg_quality: u8,
    /// `chrono` strftime template for filenames (without extension).
    pub filename_template: String,
    /// Copy each capture to the clipboard after saving.
    pub copy_to_clipboard: bool,
    /// Open the annotation editor after an in-app capture.
    pub open_editor_after_capture: bool,
    /// Show a desktop notification after each capture.
    pub show_notification: bool,
    /// Maximum number of history entries to keep.
    pub max_history: usize,
    /// Capture backend: "auto", "wlroots", "x11" or "portal".
    pub capture_backend: String,
    /// Global hotkeys.
    pub hotkeys: Hotkeys,
    /// Upload / share destinations.
    pub upload: UploadConfig,
    /// Screen recording.
    pub record: RecordConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_directory: paths::default_save_dir().to_string_lossy().into_owned(),
            image_format: ImageFormat::Png,
            jpeg_quality: 92,
            filename_template: "%Y-%m-%d_%H-%M-%S".into(),
            copy_to_clipboard: true,
            open_editor_after_capture: true,
            show_notification: true,
            max_history: 500,
            capture_backend: "auto".into(),
            hotkeys: Hotkeys::default(),
            upload: UploadConfig::default(),
            record: RecordConfig::default(),
        }
    }
}

impl Config {
    /// Load config from disk, returning defaults if it does not exist yet.
    pub fn load() -> Result<Self> {
        let path = paths::config_file();
        if !path.exists() {
            let cfg = Config::default();
            // Best-effort: persist defaults so the user has a file to edit.
            let _ = cfg.save();
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: Config = toml::from_str(&text)?;
        Ok(cfg)
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::config_file();
        paths::ensure_parent(&path)?;
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    /// Resolved, guaranteed-to-exist save directory.
    pub fn resolved_save_dir(&self) -> std::path::PathBuf {
        let raw = self.save_directory.trim();
        let dir = if raw.is_empty() {
            paths::default_save_dir()
        } else {
            let expanded = if let Some(rest) = raw.strip_prefix("~/") {
                dirs::home_dir()
                    .map(|h| h.join(rest))
                    .unwrap_or_else(|| std::path::PathBuf::from(raw))
            } else {
                std::path::PathBuf::from(raw)
            };
            expanded
        };
        let _ = std::fs::create_dir_all(&dir);
        dir
    }
}

/// Clamp quality to a sane range for the encoders.
pub fn clamp_quality(q: u8) -> u8 {
    q.clamp(1, 100)
}

/// Build a filename for a capture from the configured template.
pub fn build_filename(template: &str, format: ImageFormat) -> String {
    build_filename_with_ext(template, format.extension())
}

/// Build a filename from the configured template with an arbitrary extension.
pub fn build_filename_with_ext(template: &str, extension: &str) -> String {
    let stamp = chrono::Local::now()
        .format(if template.trim().is_empty() {
            "%Y-%m-%d_%H-%M-%S"
        } else {
            template
        })
        .to_string();
    format!("{stamp}.{extension}")
}

/// Ensure `path` is unique by appending `_1`, `_2`, ... before the extension.
pub fn unique_path(path: &Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "capture".into());
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    for i in 1..10_000 {
        let name = if ext.is_empty() {
            format!("{stem}_{i}")
        } else {
            format!("{stem}_{i}.{ext}")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_uses_configured_extension() {
        assert!(build_filename("%Y", ImageFormat::Png).ends_with(".png"));
        assert!(build_filename("%Y", ImageFormat::Jpeg).ends_with(".jpg"));
        assert!(build_filename("", ImageFormat::Webp).ends_with(".webp"));
    }

    #[test]
    fn unique_path_avoids_collisions() {
        let dir = std::env::temp_dir().join(format!("sharev-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let base = dir.join("shot.png");
        std::fs::write(&base, b"x").unwrap();

        let next = unique_path(&base);
        assert_ne!(next, base);
        assert!(next.to_string_lossy().contains("shot_1"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clamps_quality() {
        assert_eq!(clamp_quality(0), 1);
        assert_eq!(clamp_quality(200), 100);
        assert_eq!(clamp_quality(70), 70);
    }

    #[test]
    fn formats_links_for_each_copy_format() {
        let url = "https://example.com/a.png";
        assert_eq!(CopyFormat::Url.format(url), url);
        assert_eq!(CopyFormat::Markdown.format(url), "![](<https://example.com/a.png>)");
        assert_eq!(
            CopyFormat::Html.format(url),
            "<img src=\"https://example.com/a.png\" />"
        );
    }
}
