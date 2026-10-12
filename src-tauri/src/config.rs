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

    /// Infer the format from a file extension, if it is one we can write.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path
            .extension()?
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_str()
        {
            "png" => Some(ImageFormat::Png),
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::Webp),
            _ => None,
        }
    }
}

/// A single post-capture / post-upload action. Stored in the config as a
/// lowercase id and parsed leniently so that unknown ids never break loading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    Save,
    History,
    /// Copy the image itself to the clipboard.
    CopyImage,
    Editor,
    Open,
    Upload,
    Pin,
    Command,
    Notify,
    /// Copy the freshly obtained link to the clipboard (post-upload).
    CopyLink,
    /// Run OCR on the capture and copy the recognized text to the clipboard.
    Ocr,
}

impl Task {
    pub fn as_str(self) -> &'static str {
        match self {
            Task::Save => "save",
            Task::History => "history",
            Task::CopyImage => "copy",
            Task::Editor => "editor",
            Task::Open => "open",
            Task::Upload => "upload",
            Task::Pin => "pin",
            Task::Command => "command",
            Task::Notify => "notify",
            Task::CopyLink => "copyLink",
            Task::Ocr => "ocr",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace(['-', '_'], "");
        match normalized.as_str() {
            "save" => Some(Task::Save),
            "history" => Some(Task::History),
            "copy" | "copyimage" => Some(Task::CopyImage),
            "editor" | "edit" => Some(Task::Editor),
            "open" => Some(Task::Open),
            "upload" | "share" => Some(Task::Upload),
            "pin" => Some(Task::Pin),
            "command" | "action" => Some(Task::Command),
            "notify" | "notification" => Some(Task::Notify),
            "copylink" => Some(Task::CopyLink),
            "ocr" | "text" => Some(Task::Ocr),
            _ => None,
        }
    }
}

/// Ordered tasks run after a capture. When `tasks` is absent the list is derived
/// from the legacy boolean flags (see [`Config::after_capture_tasks`]).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AfterCaptureConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<String>>,
    /// Shell command run by the `command` task (the file path is substituted for
    /// `{file}` or appended; also available as `$SHAREV_FILE`).
    pub command: String,
}

/// Ordered tasks run after a successful upload.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AfterUploadConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<String>>,
}

/// Which global-hotkey mechanism to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HotkeyBackend {
    /// Portal on Wayland, `tauri-plugin-global-shortcut` elsewhere.
    #[default]
    Auto,
    /// Force the XDG portal `GlobalShortcuts` interface.
    Portal,
    /// Force `tauri-plugin-global-shortcut` (X11).
    GlobalShortcut,
    /// Do not register any hotkeys from inside the app.
    None,
}

impl HotkeyBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            HotkeyBackend::Auto => "auto",
            HotkeyBackend::Portal => "portal",
            HotkeyBackend::GlobalShortcut => "global-shortcut",
            HotkeyBackend::None => "none",
        }
    }
}

/// A single action → accelerator binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBinding {
    /// Action id, e.g. `capture-region`, `record-toggle`, `open-editor`.
    pub action: String,
    /// Accelerator, e.g. `Print`, `Ctrl+PrintScreen`.
    pub accelerator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Hotkeys {
    pub enabled: bool,
    /// Which mechanism to use: "auto", "portal", "global-shortcut" or "none".
    pub backend: HotkeyBackend,
    /// Ordered action bindings. Empty means "derive from the legacy keys".
    pub bindings: Vec<HotkeyBinding>,
    /// Legacy shortcut for interactive region capture, e.g. "PrintScreen".
    pub region: String,
    /// Legacy shortcut for fullscreen capture. Empty disables it.
    pub fullscreen: String,
    /// Legacy shortcut for active-window capture. Empty disables it.
    pub window: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: HotkeyBackend::Auto,
            bindings: Vec::new(),
            region: "PrintScreen".into(),
            fullscreen: "Ctrl+PrintScreen".into(),
            window: "Alt+PrintScreen".into(),
        }
    }
}

impl Hotkeys {
    /// The effective bindings, deriving them from the legacy keys when the
    /// binding list is empty.
    pub fn effective_bindings(&self) -> Vec<HotkeyBinding> {
        if !self.bindings.is_empty() {
            return self.bindings.clone();
        }
        let legacy = [
            ("capture-region", &self.region),
            ("capture-fullscreen", &self.fullscreen),
            ("capture-window", &self.window),
        ];
        legacy
            .into_iter()
            .filter(|(_, accel)| !accel.trim().is_empty())
            .map(|(action, accelerator)| HotkeyBinding {
                action: action.to_string(),
                accelerator: accelerator.clone(),
            })
            .collect()
    }
}

/// Default post-capture tasks, matching the legacy default booleans.
pub fn default_capture_tasks() -> Vec<String> {
    ["save", "history", "copy", "editor", "notify"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Default post-upload tasks.
pub fn default_upload_tasks() -> Vec<String> {
    ["copyLink", "notify"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Which destination uploaded images are sent to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UploaderKind {
    #[default]
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

/// How the returned link is formatted before being copied to the clipboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CopyFormat {
    #[default]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordFormat {
    #[default]
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

/// Which audio sources a recording captures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioMode {
    #[default]
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

/// Optional feature flags. A feature that is off never requires its system
/// dependency to be installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Features {
    /// Optical character recognition (requires Tesseract).
    pub ocr: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self { ocr: true }
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
    /// Seconds to wait before capturing / recording (0 = no delay).
    pub capture_delay_seconds: u32,
    /// Target monitor name for fullscreen capture/recording ("" = focused).
    pub monitor: String,
    /// Legacy: copy each capture to the clipboard after saving. Kept for
    /// migration only; the `copy` task supersedes it.
    pub copy_to_clipboard: bool,
    /// Legacy: open the annotation editor after an in-app capture.
    pub open_editor_after_capture: bool,
    /// Legacy: show a desktop notification after each capture.
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
    /// Ordered tasks run after a capture. Field-level `default` keeps a missing
    /// section as `None` so legacy boolean flags can drive migration.
    #[serde(default)]
    pub after_capture: AfterCaptureConfig,
    /// Ordered tasks run after a successful upload.
    #[serde(default)]
    pub after_upload: AfterUploadConfig,
    /// OCR language (Tesseract language code, e.g. "eng").
    pub ocr_language: String,
    /// Toggleable features with optional system dependencies.
    pub features: Features,
    /// Image effects pipeline.
    pub effects: crate::effects::EffectsConfig,
    /// Scrolling capture settings.
    pub scroll: crate::scroll::ScrollConfig,
    /// Timed / change-detection auto capture.
    pub auto: crate::auto::AutoCaptureConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            save_directory: paths::default_save_dir().to_string_lossy().into_owned(),
            image_format: ImageFormat::Png,
            jpeg_quality: 92,
            filename_template: "%Y-%m-%d_%H-%M-%S".into(),
            capture_delay_seconds: 0,
            monitor: String::new(),
            copy_to_clipboard: true,
            open_editor_after_capture: true,
            show_notification: true,
            max_history: 500,
            capture_backend: "auto".into(),
            hotkeys: Hotkeys::default(),
            upload: UploadConfig::default(),
            record: RecordConfig::default(),
            after_capture: AfterCaptureConfig {
                tasks: Some(default_capture_tasks()),
                command: String::new(),
            },
            after_upload: AfterUploadConfig {
                tasks: Some(default_upload_tasks()),
            },
            ocr_language: "eng".into(),
            features: Features::default(),
            effects: crate::effects::EffectsConfig::default(),
            scroll: crate::scroll::ScrollConfig::default(),
            auto: crate::auto::AutoCaptureConfig::default(),
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
        let mut cfg: Config = toml::from_str(&text)?;
        cfg.normalize();
        Ok(cfg)
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }

    /// Fill in any derived values (task chains, hotkey bindings) that were
    /// absent from an older config file.
    pub fn normalize(&mut self) {
        if self.after_capture.tasks.is_none() {
            self.after_capture.tasks = Some(self.derive_capture_tasks());
        }
        if self.after_upload.tasks.is_none() {
            self.after_upload.tasks = Some(default_upload_tasks());
        }
        if self.hotkeys.bindings.is_empty() {
            self.hotkeys.bindings = self.hotkeys.effective_bindings();
        }
    }

    /// Derive a post-capture task list from the legacy boolean flags.
    fn derive_capture_tasks(&self) -> Vec<String> {
        let mut tasks = vec!["save".to_string(), "history".to_string()];
        if self.copy_to_clipboard {
            tasks.push("copy".into());
        }
        if self.open_editor_after_capture {
            tasks.push("editor".into());
        }
        if self.show_notification {
            tasks.push("notify".into());
        }
        tasks
    }

    /// The ordered post-capture tasks, migrating legacy configs on the fly.
    pub fn after_capture_tasks(&self) -> Vec<Task> {
        let raw = self
            .after_capture
            .tasks
            .clone()
            .unwrap_or_else(|| self.derive_capture_tasks());
        raw.iter().filter_map(|id| Task::parse(id)).collect()
    }

    /// The ordered post-upload tasks.
    pub fn after_upload_tasks(&self) -> Vec<Task> {
        let raw = self
            .after_upload
            .tasks
            .clone()
            .unwrap_or_else(default_upload_tasks);
        raw.iter().filter_map(|id| Task::parse(id)).collect()
    }

    /// Whether a capture task is enabled.
    pub fn has_capture_task(&self, task: Task) -> bool {
        self.after_capture_tasks().contains(&task)
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
        } else if let Some(rest) = raw.strip_prefix("~/") {
            dirs::home_dir()
                .map(|h| h.join(rest))
                .unwrap_or_else(|| std::path::PathBuf::from(raw))
        } else {
            std::path::PathBuf::from(raw)
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
        assert_eq!(
            CopyFormat::Markdown.format(url),
            "![](<https://example.com/a.png>)"
        );
        assert_eq!(
            CopyFormat::Html.format(url),
            "<img src=\"https://example.com/a.png\" />"
        );
    }

    #[test]
    fn parses_task_ids_leniently() {
        assert_eq!(Task::parse("save"), Some(Task::Save));
        assert_eq!(Task::parse("  Notify "), Some(Task::Notify));
        assert_eq!(Task::parse("copy-image"), Some(Task::CopyImage));
        assert_eq!(Task::parse("copyLink"), Some(Task::CopyLink));
        assert_eq!(Task::parse("nonsense"), None);
    }

    #[test]
    fn migrates_legacy_booleans_into_tasks() {
        // Simulate an old config: the task list is absent.
        let text = r#"
            copyToClipboard = true
            openEditorAfterCapture = false
            showNotification = false
        "#;
        let mut cfg: Config = toml::from_str(text).unwrap();
        assert!(cfg.after_capture.tasks.is_none());
        cfg.normalize();

        let tasks = cfg.after_capture_tasks();
        assert_eq!(tasks, vec![Task::Save, Task::History, Task::CopyImage]);
    }

    #[test]
    fn explicit_task_list_wins_over_legacy_booleans() {
        let text = r#"
            copyToClipboard = false
            [afterCapture]
            tasks = ["save", "pin", "editor"]
        "#;
        let mut cfg: Config = toml::from_str(text).unwrap();
        cfg.normalize();
        assert_eq!(
            cfg.after_capture_tasks(),
            vec![Task::Save, Task::Pin, Task::Editor]
        );
    }

    #[test]
    fn derives_hotkey_bindings_from_legacy_keys() {
        let text = r#"
            [hotkeys]
            region = "Print"
            fullscreen = ""
            window = "Alt+Print"
        "#;
        let mut cfg: Config = toml::from_str(text).unwrap();
        cfg.normalize();
        let bindings = cfg.hotkeys.effective_bindings();
        let actions: Vec<_> = bindings.iter().map(|b| b.action.as_str()).collect();
        assert_eq!(actions, vec!["capture-region", "capture-window"]);
    }

    #[test]
    fn config_round_trips_through_toml() {
        let cfg = Config::default();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed.after_capture_tasks(), cfg.after_capture_tasks());
        assert_eq!(parsed.after_upload_tasks(), cfg.after_upload_tasks());
    }
}
