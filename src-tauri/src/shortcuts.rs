//! Global hotkeys.
//!
//! Two mechanisms are supported and selectable from the config:
//!
//! * `tauri-plugin-global-shortcut` — works on X11 (and partially through
//!   XWayland).
//! * The XDG Desktop Portal `GlobalShortcuts` interface — the portable path on
//!   Wayland (implemented in [`crate::portal_shortcuts`]).
//!
//! `auto` picks the portal on Wayland and the plugin elsewhere.

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::capture::CaptureKind;
use crate::config::{Config, HotkeyBackend, HotkeyBinding};
use crate::platform::display::{DisplayServer, detect};

/// A user-triggerable action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    CaptureRegion,
    CaptureFullscreen,
    CaptureWindow,
    CaptureScrolling,
    CaptureLastRegion,
    RecordToggle,
    StopRecording,
    AbortRecording,
    AutoCaptureToggle,
    UploadLast,
    PinLast,
    OpenEditor,
}

impl Action {
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace([' ', '_'], "-");
        match normalized.as_str() {
            "capture-region" | "capture" | "region" => Some(Action::CaptureRegion),
            "capture-fullscreen" | "fullscreen" => Some(Action::CaptureFullscreen),
            "capture-window" | "window" => Some(Action::CaptureWindow),
            "capture-scrolling" | "scrolling" | "scroll" => Some(Action::CaptureScrolling),
            "capture-last-region" | "last-region" => Some(Action::CaptureLastRegion),
            "record-toggle" | "record" | "toggle-recording" => Some(Action::RecordToggle),
            "stop-recording" => Some(Action::StopRecording),
            "abort-recording" => Some(Action::AbortRecording),
            "auto-capture-toggle" | "auto-capture" | "toggle-auto-capture" => {
                Some(Action::AutoCaptureToggle)
            }
            "upload-last" | "upload" => Some(Action::UploadLast),
            "pin-last" | "pin" => Some(Action::PinLast),
            "open-editor" | "editor" => Some(Action::OpenEditor),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Action::CaptureRegion => "capture-region",
            Action::CaptureFullscreen => "capture-fullscreen",
            Action::CaptureWindow => "capture-window",
            Action::CaptureScrolling => "capture-scrolling",
            Action::CaptureLastRegion => "capture-last-region",
            Action::RecordToggle => "record-toggle",
            Action::StopRecording => "stop-recording",
            Action::AbortRecording => "abort-recording",
            Action::AutoCaptureToggle => "auto-capture-toggle",
            Action::UploadLast => "upload-last",
            Action::PinLast => "pin-last",
            Action::OpenEditor => "open-editor",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Action::CaptureRegion => "Capture a region",
            Action::CaptureFullscreen => "Capture the screen",
            Action::CaptureWindow => "Capture a window",
            Action::CaptureScrolling => "Capture a scrolling region",
            Action::CaptureLastRegion => "Capture the last region again",
            Action::RecordToggle => "Start/stop recording",
            Action::StopRecording => "Stop the recording",
            Action::AbortRecording => "Discard the recording",
            Action::AutoCaptureToggle => "Start/stop auto capture",
            Action::UploadLast => "Upload the last capture",
            Action::PinLast => "Pin the last capture",
            Action::OpenEditor => "Open ShareV",
        }
    }
}

fn active_backend_slot() -> &'static Mutex<Option<&'static str>> {
    static SLOT: Mutex<Option<&'static str>> = Mutex::new(None);
    &SLOT
}

/// Record which mechanism is currently responsible for hotkeys.
pub fn set_backend(name: &'static str) {
    *active_backend_slot().lock().unwrap() = Some(name);
}

/// The mechanism currently responsible for hotkeys ("portal", "global-shortcut"
/// or "none").
pub fn active_backend() -> String {
    active_backend_slot()
        .lock()
        .unwrap()
        .unwrap_or("none")
        .to_string()
}

/// Whether the XDG portal GlobalShortcuts interface is currently bound.
pub fn portal_available() -> bool {
    crate::portal_shortcuts::available()
}

/// Drop all registered hotkeys and register the ones from `config`.
pub fn reload(app: &AppHandle, config: &Config) {
    if let Err(err) = app.global_shortcut().unregister_all() {
        log::warn!("could not clear previous hotkeys: {err}");
    }
    crate::portal_shortcuts::stop();

    if !config.hotkeys.enabled {
        set_backend("none");
        return;
    }

    let bindings = config.hotkeys.effective_bindings();
    if bindings.is_empty() {
        set_backend("none");
        return;
    }

    let wayland = detect() == DisplayServer::Wayland;
    let choice = match config.hotkeys.backend {
        HotkeyBackend::None => "none",
        HotkeyBackend::GlobalShortcut => "global-shortcut",
        HotkeyBackend::Portal => "portal",
        HotkeyBackend::Auto => {
            if wayland {
                "portal"
            } else {
                "global-shortcut"
            }
        }
    };

    match choice {
        "portal" => {
            set_backend("portal");
            let fallback = config.hotkeys.backend == HotkeyBackend::Auto;
            crate::portal_shortcuts::start(app.clone(), bindings, fallback);
        }
        "global-shortcut" => {
            set_backend("global-shortcut");
            register_plugin(app, bindings);
        }
        _ => set_backend("none"),
    }
}

/// Re-register the plugin bindings from the current config (used when the portal
/// turns out to be unavailable).
pub fn register_from_config(app: &AppHandle) {
    let config = app
        .state::<crate::AppState>()
        .config
        .lock()
        .unwrap()
        .clone();
    register_plugin(app, config.hotkeys.effective_bindings());
}

fn register_plugin(app: &AppHandle, bindings: Vec<HotkeyBinding>) {
    for binding in bindings {
        let accelerator = binding.accelerator.trim();
        if accelerator.is_empty() {
            continue;
        }
        let Some(action) = Action::parse(&binding.action) else {
            log::warn!("unknown hotkey action '{}'", binding.action);
            continue;
        };
        let shortcut: Shortcut = match accelerator.parse() {
            Ok(shortcut) => shortcut,
            Err(err) => {
                log::warn!("invalid hotkey '{accelerator}': {err}");
                continue;
            }
        };

        let app_handle = app.clone();
        let result = app
            .global_shortcut()
            .on_shortcut(shortcut, move |_app, _shortcut, event| {
                if event.state() != ShortcutState::Pressed {
                    return;
                }
                dispatch(&app_handle, action);
            });

        if let Err(err) = result {
            log::warn!(
                "could not register hotkey '{accelerator}' ({err}). On Wayland, bind \
                 `sharev capture <region|fullscreen|window>` in your compositor or \
                 enable the portal backend."
            );
        }
    }
}

/// Run an action on the async runtime. Shared by the plugin and the portal.
pub fn dispatch(app: &AppHandle, action: Action) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match action {
            Action::CaptureRegion => {
                crate::commands::capture_and_notify(app, CaptureKind::Region).await
            }
            Action::CaptureFullscreen => {
                crate::commands::capture_and_notify(app, CaptureKind::Fullscreen).await
            }
            Action::CaptureWindow => {
                crate::commands::capture_and_notify(app, CaptureKind::Window).await
            }
            Action::CaptureScrolling => {
                crate::commands::capture_and_notify(app, CaptureKind::Scrolling).await
            }
            Action::CaptureLastRegion => {
                crate::commands::capture_last_region_and_notify(app).await
            }
            Action::RecordToggle => {
                crate::commands::toggle_recording(app, CaptureKind::Region).await
            }
            Action::StopRecording => {
                let active = app
                    .state::<crate::AppState>()
                    .recorder
                    .lock()
                    .unwrap()
                    .take();
                if let Some(active) = active {
                    let config = app
                        .state::<crate::AppState>()
                        .config
                        .lock()
                        .unwrap()
                        .clone();
                    match crate::commands::stop_and_store(active, &config).await {
                        Ok(result) => {
                            let _ = app.emit("recording-stopped", &result);
                        }
                        Err(err) => {
                            log::warn!("recording failed: {err}");
                            crate::commands::notify_error(&app, &err.to_string());
                        }
                    }
                    crate::tray::refresh(&app);
                }
            }
            Action::AbortRecording => {
                if let Err(err) = crate::commands::abort_recording(
                    app.clone(),
                    app.state::<crate::AppState>(),
                )
                .await
                {
                    log::warn!("could not abort recording: {err}");
                }
            }
            Action::AutoCaptureToggle => {
                let active = app
                    .state::<crate::AppState>()
                    .auto
                    .lock()
                    .unwrap()
                    .is_some();
                if active {
                    if let Err(err) = crate::commands::stop_auto_capture(
                        app.state::<crate::AppState>(),
                    ) {
                        log::warn!("could not stop auto capture: {err}");
                    }
                } else if let Err(err) = crate::commands::start_auto_capture(
                    app.clone(),
                    app.state::<crate::AppState>(),
                    "region".to_string(),
                    Some(3000),
                    Some(false),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                {
                    log::warn!("could not start auto capture: {err}");
                    crate::commands::capture_error(&app, &err.to_string());
                }
                crate::tray::refresh(&app);
            }
            Action::UploadLast => match crate::commands::upload_last(app.clone()).await {
                Ok(Some(outcome)) => {
                    crate::commands::notify_info(&app, "Upload finished", &outcome.url);
                }
                Ok(None) => {}
                Err(err) => {
                    log::warn!("could not upload the last capture: {err}");
                    crate::commands::capture_error(&app, &err.to_string());
                }
            },
            Action::PinLast => {
                if let Err(err) = crate::commands::pin_last(app.clone()) {
                    log::warn!("could not pin the last capture: {err}");
                    crate::commands::capture_error(&app, &err.to_string());
                }
            }
            Action::OpenEditor => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                let _ = app.emit("open-editor", ());
            }
        }
    });
}

/// Translate a common accelerator into the XDG "shortcuts" trigger syntax used
/// by the portal. Returns `None` when the shape is not recognised.
pub fn to_portal_trigger(accelerator: &str) -> Option<String> {
    let accelerator = accelerator.trim();
    if accelerator.is_empty() {
        return None;
    }
    if accelerator.eq_ignore_ascii_case("printscreen") || accelerator.eq_ignore_ascii_case("print")
    {
        return Some("Print".to_string());
    }

    let mut modifiers: Vec<&str> = Vec::new();
    let mut key: Option<String> = None;
    for part in accelerator.split('+') {
        let part = part.trim();
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers.push("<Control>"),
            "alt" => modifiers.push("<Alt>"),
            "shift" => modifiers.push("<Shift>"),
            "super" | "meta" | "win" | "cmd" => modifiers.push("<Super>"),
            other => {
                if key.is_some() {
                    return None;
                }
                key = Some(normalize_key(other));
            }
        }
    }

    let key = key?;
    let mut trigger = modifiers.join("");
    trigger.push_str(&key);
    Some(trigger)
}

fn normalize_key(key: &str) -> String {
    match key {
        "printscreen" => "Print".to_string(),
        other if other.chars().count() == 1 => other.to_ascii_uppercase(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_action_ids() {
        assert_eq!(Action::parse("capture-region"), Some(Action::CaptureRegion));
        assert_eq!(
            Action::parse("capture-scrolling"),
            Some(Action::CaptureScrolling)
        );
        assert_eq!(Action::parse(" Record-Toggle "), Some(Action::RecordToggle));
        assert_eq!(Action::parse("open-editor"), Some(Action::OpenEditor));
        assert_eq!(Action::parse("last-region"), Some(Action::CaptureLastRegion));
        assert_eq!(
            Action::parse("auto-capture-toggle"),
            Some(Action::AutoCaptureToggle)
        );
        assert_eq!(Action::parse("stop-recording"), Some(Action::StopRecording));
        assert_eq!(Action::parse("upload-last"), Some(Action::UploadLast));
        assert_eq!(Action::parse("pin-last"), Some(Action::PinLast));
        assert_eq!(Action::parse("bogus"), None);
    }

    #[test]
    fn translates_portal_triggers() {
        assert_eq!(to_portal_trigger("PrintScreen").as_deref(), Some("Print"));
        assert_eq!(to_portal_trigger("Ctrl+P").as_deref(), Some("<Control>P"));
        assert_eq!(
            to_portal_trigger("Ctrl+Shift+A").as_deref(),
            Some("<Control><Shift>A")
        );
        assert_eq!(to_portal_trigger("").as_deref(), None);
    }
}
