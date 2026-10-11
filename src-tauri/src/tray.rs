use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::capture::CaptureKind;

/// Build the system tray icon and its menu.
pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let region = MenuItem::with_id(app, "capture_region", "Capture Region", true, None::<&str>)?;
    let fullscreen =
        MenuItem::with_id(app, "capture_fullscreen", "Capture Fullscreen", true, None::<&str>)?;
    let window = MenuItem::with_id(app, "capture_window", "Capture Window", true, None::<&str>)?;
    let open_folder = MenuItem::with_id(app, "open_folder", "Open Save Folder", true, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show ShareV", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep_a = PredefinedMenuItem::separator(app)?;
    let sep_b = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &region,
            &fullscreen,
            &window,
            &sep_a,
            &open_folder,
            &sep_b,
            &show,
            &quit,
        ],
    )?;

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

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "capture_region" => spawn_capture(app, CaptureKind::Region),
        "capture_fullscreen" => spawn_capture(app, CaptureKind::Fullscreen),
        "capture_window" => spawn_capture(app, CaptureKind::Window),
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
