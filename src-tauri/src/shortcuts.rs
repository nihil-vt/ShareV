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
    RecordToggle,
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
            "record-toggle" | "record" | "toggle-recording" => Some(Action::RecordToggle),
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
            Action::RecordToggle => "record-toggle",
            Action::OpenEditor => "open-editor",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Action::CaptureRegion => "Capture a region",
            Action::CaptureFullscreen => "Capture the screen",
            Action::CaptureWindow => "Capture a window",
            Action::CaptureScrolling => "Capture a scrolling region",
            Action::RecordToggle => "Start/stop recording",
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
            Action::RecordToggle => {
                crate::commands::toggle_recording(app, CaptureKind::Region).await
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
