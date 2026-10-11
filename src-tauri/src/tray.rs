use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

use crate::capture::CaptureKind;

/// Build the tray menu, reflecting the current recording and history state.
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let recording = app
        .state::<crate::AppState>()
        .recorder
        .lock()
        .unwrap()
        .is_some();
    let has_last = !crate::history::list(Some(1)).unwrap_or_default().is_empty();

    let region = MenuItem::with_id(app, "capture_region", "Capture Region", true, None::<&str>)?;
    let fullscreen =
        MenuItem::with_id(app, "capture_fullscreen", "Capture Fullscreen", true, None::<&str>)?;
    let window = MenuItem::with_id(app, "capture_window", "Capture Window", true, None::<&str>)?;
    let toggle_label = if recording {
        "Stop Recording"
    } else {
        "Start Recording"
    };
    let record = MenuItem::with_id(app, "toggle_recording", toggle_label, true, None::<&str>)?;
    let pin = MenuItem::with_id(
        app,
        "pin_last",
        "Pin Last Capture",
        has_last && !recording,
        None::<&str>,
    )?;
    let upload = MenuItem::with_id(
        app,
        "upload_last",
        "Upload Last Capture",
        has_last && !recording,
        None::<&str>,
    )?;
    let open_folder = MenuItem::with_id(app, "open_folder", "Open Save Folder", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show ShareV", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep_a = PredefinedMenuItem::separator(app)?;
    let sep_b = PredefinedMenuItem::separator(app)?;
    let sep_c = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &region,
            &fullscreen,
            &window,
            &sep_a,
            &record,
            &sep_b,
            &pin,
            &upload,
            &sep_c,
            &open_folder,
            &show,
            &quit,
        ],
    )
}

/// Build the system tray icon and its menu.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("ShareV")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the tray menu to reflect the current state (recording, history).
pub fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    match build_menu(app) {
        Ok(menu) => {
            if let Err(err) = tray.set_menu(Some(menu)) {
                log::warn!("could not refresh tray menu: {err}");
            }
        }
        Err(err) => log::warn!("could not build tray menu: {err}"),
    }
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "capture_region" => spawn_capture(app, CaptureKind::Region),
        "capture_fullscreen" => spawn_capture(app, CaptureKind::Fullscreen),
        "capture_window" => spawn_capture(app, CaptureKind::Window),
        "toggle_recording" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::commands::toggle_recording(app, CaptureKind::Region).await;
            });
        }
        "pin_last" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = crate::commands::pin_last(app.clone()) {
                    log::warn!("could not pin the last capture: {err}");
                    crate::commands::capture_error(&app, &err.to_string());
                }
            });
        }
        "upload_last" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = crate::commands::upload_last(app.clone()).await {
                    log::warn!("could not upload the last capture: {err}");
                    crate::commands::capture_error(&app, &err.to_string());
                }
            });
        }
        "open_folder" => {
            let dir = app
                .state::<crate::AppState>()
                .config
                .lock()
                .unwrap()
                .resolved_save_dir();
            let _ = crate::commands::open_with(&dir.to_string_lossy());
        }
        "show" => show_main_window(app),
        "quit" => app.exit(0),
        _ => {}
    }
}

fn spawn_capture(app: &AppHandle, kind: CaptureKind) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::commands::capture_and_notify(app, kind).await;
    });
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
