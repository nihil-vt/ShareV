//! XDG Desktop Portal `GlobalShortcuts` integration.
//!
//! This is the portable way to register system-wide shortcuts on Wayland, where
//! `XGrabKey`-style grabbing does not work. It is inherently optional: when the
//! portal (or a session bus) is missing we log a warning and let the caller fall
//! back to the compositor's own key binding or the plugin backend.
//!
//! The interface is stateful: a session is created, shortcuts are bound to it,
//! and an `Activated` signal is delivered whenever the user presses one. We keep
//! the session alive in a global slot and listen for activations on a background
//! task. A generation counter lets a freshly-reloaded configuration cancel the
//! previous listener.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use ashpd::desktop::{CreateSessionOptions, Session};
use futures_util::StreamExt;
use tauri::AppHandle;

use crate::config::HotkeyBinding;
use crate::error::{Error, Result};
use crate::shortcuts::{self, Action};

/// Whether a portal session is currently bound.
static AVAILABLE: AtomicBool = AtomicBool::new(false);
/// Incremented on every (re)load; stale listener tasks observe the change and
/// stop dispatching.
static GENERATION: AtomicU64 = AtomicU64::new(0);

struct Active {
    session: Session<GlobalShortcuts>,
}

fn state() -> &'static Mutex<Option<Active>> {
    static STATE: OnceLock<Mutex<Option<Active>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

/// Whether the portal backend managed to bind its shortcuts.
pub fn available() -> bool {
    AVAILABLE.load(Ordering::Relaxed)
}

/// Start a portal session for `bindings`. When `fallback` is set, failure drops
/// back to the plugin backend.
pub fn start(app: AppHandle, bindings: Vec<HotkeyBinding>, fallback: bool) {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    tauri::async_runtime::spawn(async move {
        match setup(&app, &bindings, generation).await {
            Ok(session) => {
                AVAILABLE.store(true, Ordering::Relaxed);
                shortcuts::set_backend("portal");
                // Close any previous session instead of dropping it silently.
                let previous = state().lock().unwrap().replace(Active { session });
                if let Some(previous) = previous
                    && let Err(err) = previous.session.close().await
                {
                    log::warn!("could not close the previous portal session: {err}");
                }
            }
            Err(err) => {
                AVAILABLE.store(false, Ordering::Relaxed);
                log::warn!("XDG portal global shortcuts unavailable: {err}");
                if fallback {
                    shortcuts::set_backend("global-shortcut");
                    shortcuts::register_from_config(&app);
                } else {
                    shortcuts::set_backend("none");
                }
            }
        }
    });
}

/// Close any active portal session.
pub fn stop() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    let previous = state().lock().unwrap().take();
    if let Some(active) = previous {
        tauri::async_runtime::spawn(async move {
            if let Err(err) = active.session.close().await {
                log::warn!("could not close the portal session: {err}");
            }
        });
    }
    AVAILABLE.store(false, Ordering::Relaxed);
}

async fn setup(
    app: &AppHandle,
    bindings: &[HotkeyBinding],
    generation: u64,
) -> Result<Session<GlobalShortcuts>> {
    let shortcuts = GlobalShortcuts::new().await.map_err(map_err)?;
    let session = shortcuts
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(map_err)?;

    let mut descriptors = Vec::new();
    let mut routes: HashMap<String, Action> = HashMap::new();
    for (index, binding) in bindings.iter().enumerate() {
        let Some(action) = Action::parse(&binding.action) else {
            continue;
        };
        let id = format!("{}-{index}", action.id());
        let mut descriptor = NewShortcut::new(id.clone(), action.description());
        if let Some(trigger) = shortcuts::to_portal_trigger(&binding.accelerator) {
            descriptor = descriptor.preferred_trigger(trigger.as_str());
        }
        descriptors.push(descriptor);
        routes.insert(id, action);
    }

    if descriptors.is_empty() {
        return Err(Error::msg("no valid hotkey bindings to register"));
    }

    let request = shortcuts
        .bind_shortcuts(&session, &descriptors, None, Default::default())
        .await
        .map_err(map_err)?;
    // Wait for the user to finish the portal's binding dialog.
    let _bound = request.response().map_err(map_err)?;

    let mut stream = shortcuts.receive_activated().await.map_err(map_err)?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        while let Some(activated) = stream.next().await {
            if generation != GENERATION.load(Ordering::SeqCst) {
                break;
            }
            if let Some(action) = routes.get(activated.shortcut_id()) {
                shortcuts::dispatch(&app, *action);
            }
        }
    });

    Ok(session)
}

fn map_err(err: ashpd::Error) -> Error {
    Error::msg(format!("portal: {err}"))
}
