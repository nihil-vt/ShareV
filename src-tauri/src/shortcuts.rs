use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::capture::CaptureKind;
use crate::config::Config;

/// Drop all registered hotkeys and register the ones from `config`.
pub fn reload(app: &AppHandle, config: &Config) {
    let shortcuts = app.global_shortcut();
    if let Err(err) = shortcuts.unregister_all() {
        log::warn!("could not clear previous hotkeys: {err}");
    }

    if !config.hotkeys.enabled {
        return;
    }

    register_one(app, &config.hotkeys.region, "region");
    register_one(app, &config.hotkeys.fullscreen, "fullscreen");
    register_one(app, &config.hotkeys.window, "window");
}

fn register_one(app: &AppHandle, binding: &str, kind: &'static str) {
    let binding = binding.trim();
    if binding.is_empty() {
        return;
    }

    let shortcut: Shortcut = match binding.parse() {
        Ok(shortcut) => shortcut,
        Err(err) => {
            log::warn!("invalid hotkey '{binding}': {err}");
            return;
        }
    };

    let app_handle = app.clone();
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            let app = app_handle.clone();
            let kind = kind.to_string();
            tauri::async_runtime::spawn(async move {
                if let Ok(capture_kind) = CaptureKind::parse(&kind) {
                    crate::commands::capture_and_notify(app, capture_kind).await;
                }
            });
        });

    if let Err(err) = result {
        log::warn!(
            "could not register hotkey '{binding}' ({err}). On Wayland, bind \
             `sharev capture <region|fullscreen|window>` in your compositor instead."
        );
    }
}
