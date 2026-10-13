use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
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
    let fullscreen = MenuItem::with_id(
        app,
        "capture_fullscreen",
        "Capture Fullscreen",
        true,
        None::<&str>,
    )?;
    let window = MenuItem::with_id(app, "capture_window", "Capture Window", true, None::<&str>)?;
    let scrolling = MenuItem::with_id(
        app,
        "capture_scrolling",
        "Capture Scrolling",
        true,
        None::<&str>,
    )?;
    let toggle_label = if recording {
        "Stop Recording"
    } else {
        "Start Recording"
    };
    let record = MenuItem::with_id(app, "toggle_recording", toggle_label, true, None::<&str>)?;
    let auto_active = app
        .state::<crate::AppState>()
        .auto
        .lock()
        .unwrap()
        .is_some();
    let auto_label = if auto_active {
        "Stop Auto Capture"
    } else {
        "Start Auto Capture"
    };
    let auto = MenuItem::with_id(app, "toggle_auto_capture", auto_label, true, None::<&str>)?;
    let hotkeys_on = app
        .state::<crate::AppState>()
        .config
        .lock()
        .unwrap()
        .hotkeys
        .enabled;
    let hotkeys_label = if hotkeys_on {
        "Disable Hotkeys"
    } else {
        "Enable Hotkeys"
    };
    let hotkeys = MenuItem::with_id(app, "toggle_hotkeys", hotkeys_label, true, None::<&str>)?;
    let current_delay = app
        .state::<crate::AppState>()
        .config
        .lock()
        .unwrap()
        .capture_delay_seconds;
    let delay_menu = Submenu::with_id(app, "delay_menu", "Capture Delay", true)?;
    for seconds in [0u32, 1, 3, 5, 10] {
        let label = if seconds == 0 {
            "No delay".to_string()
        } else {
            format!("{seconds}s")
        };
        let marker = if current_delay == seconds { "● " } else { "" };
        delay_menu.append(&MenuItem::with_id(
            app,
            format!("delay_{seconds}"),
            format!("{marker}{label}"),
            true,
            None::<&str>,
        )?)?;
    }
    let current_uploader = app
        .state::<crate::AppState>()
        .config
        .lock()
        .unwrap()
        .upload
        .uploader;
    let dest_menu = Submenu::with_id(app, "dest_menu", "Image Uploader", true)?;
    for (id, label) in [
        ("none", "None (local only)"),
        ("imgur", "Imgur"),
        ("http", "Custom HTTP"),
        ("command", "Custom command"),
    ] {
        let marker = if current_uploader.as_str() == id {
            "● "
        } else {
            ""
        };
        dest_menu.append(&MenuItem::with_id(
            app,
            format!("uploader_{id}"),
            format!("{marker}{label}"),
            true,
            None::<&str>,
        )?)?;
    }
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
    let open_folder =
        MenuItem::with_id(app, "open_folder", "Open Save Folder", true, None::<&str>)?;
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
            &scrolling,
            &sep_a,
            &record,
            &auto,
            &hotkeys,
            &delay_menu,
            &dest_menu,
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
        "capture_scrolling" => spawn_capture(app, CaptureKind::Scrolling),
        "toggle_recording" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::commands::toggle_recording(app, CaptureKind::Region).await;
            });
        }
        "toggle_auto_capture" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let active = app
                    .state::<crate::AppState>()
                    .auto
                    .lock()
                    .unwrap()
                    .is_some();
                if active {
                    if let Err(err) =
                        crate::commands::stop_auto_capture(app.state::<crate::AppState>())
                    {
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
            });
        }
        "toggle_hotkeys" => {
            let mut config = app
                .state::<crate::AppState>()
                .config
                .lock()
                .unwrap()
                .clone();
            config.hotkeys.enabled = !config.hotkeys.enabled;
            match config.save() {
                Ok(()) => {
                    *app.state::<crate::AppState>().config.lock().unwrap() = config.clone();
                    crate::shortcuts::reload(app, &config);
                }
                Err(err) => log::warn!("could not save hotkey setting: {err}"),
            }
            crate::tray::refresh(app);
        }
        id if id.starts_with("delay_") => {
            if let Ok(seconds) = id["delay_".len()..].parse::<u32>() {
                let mut config = app
                    .state::<crate::AppState>()
                    .config
                    .lock()
                    .unwrap()
                    .clone();
                config.capture_delay_seconds = seconds;
                match config.save() {
                    Ok(()) => {
                        *app.state::<crate::AppState>().config.lock().unwrap() = config;
                    }
                    Err(err) => log::warn!("could not save delay setting: {err}"),
                }
                crate::tray::refresh(app);
            }
        }
        id if id.starts_with("uploader_") => {
            if let Some(uploader) =
                crate::config::UploaderKind::parse(&id["uploader_".len()..])
            {
                let mut config = app
                    .state::<crate::AppState>()
                    .config
                    .lock()
                    .unwrap()
                    .clone();
                config.upload.uploader = uploader;
                match config.save() {
                    Ok(()) => {
                        *app.state::<crate::AppState>().config.lock().unwrap() = config;
                    }
                    Err(err) => log::warn!("could not save uploader setting: {err}"),
                }
                crate::tray::refresh(app);
            }
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
