//! Headless command-line interface.
//!
//! When ShareV is launched with arguments it behaves like a small CLI so it can
//! be bound to compositor / desktop-environment shortcuts, e.g. in Hyprland:
//!
//! ```text
//! bind = , PRINT, exec, sharev capture region
//! bind = SHIFT, PRINT, exec, sharev capture fullscreen
//! bind = , SUPER SHIFT, R, exec, sharev record region
//! ```

use crate::capture::CaptureKind;
use crate::config::Config;
use crate::error::{Error, Result};

/// Returns `Ok(Some(code))` to exit with a status, or `Ok(None)` to launch the GUI.
pub fn try_run(args: &[String]) -> Result<Option<i32>> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(None);
    };

    match command {
        "-h" | "--help" | "help" => {
            print_help();
            Ok(Some(0))
        }
        "-V" | "--version" | "version" => {
            println!("sharev {}", env!("CARGO_PKG_VERSION"));
            Ok(Some(0))
        }
        "capture" => run_capture(args.get(1).map(String::as_str).unwrap_or("region")),
        "capture-region" => run_capture("region"),
        "capture-fullscreen" => run_capture("fullscreen"),
        "capture-window" => run_capture("window"),
        "upload" => run_upload(args.get(1).map(String::as_str)),
        "record" => run_record(args.get(1).map(String::as_str).unwrap_or("region")),
        "record-region" => run_record("region"),
        "record-fullscreen" => run_record("fullscreen"),
        "record-window" => run_record("window"),
        "diagnostics" | "doctor" => {
            print_diagnostics();
            Ok(Some(0))
        }
        other => {
            eprintln!("sharev: unknown command '{other}'\n");
            print_help();
            Ok(Some(2))
        }
    }
}

fn run_capture(kind: &str) -> Result<Option<i32>> {
    let capture_kind = match CaptureKind::parse(kind) {
        Ok(kind) => kind,
        Err(err) => {
            eprintln!("sharev: {err}");
            return Ok(Some(2));
        }
    };

    let config = Config::load_or_default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| Error::msg(err.to_string()))?;

    match runtime.block_on(crate::pipeline::perform(&config, capture_kind, None)) {
        Ok(result) => {
            // Print just the path so scripts can consume it.
            println!("{}", result.path);
            Ok(Some(0))
        }
        Err(Error::Cancelled) => {
            eprintln!("sharev: capture cancelled");
            Ok(Some(1))
        }
        Err(err) => {
            eprintln!("sharev: capture failed: {err}");
            Ok(Some(1))
        }
    }
}

fn run_upload(path: Option<&str>) -> Result<Option<i32>> {
    let Some(path) = path else {
        eprintln!("sharev: upload needs a file path");
        return Ok(Some(2));
    };

    let config = Config::load_or_default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| Error::msg(err.to_string()))?;

    match runtime.block_on(crate::upload::upload(&config, std::path::Path::new(path))) {
        Ok(url) => {
            let link = config.upload.copy_format.format(&url);
            if crate::clipboard::copy_text(&link).is_err() {
                eprintln!("sharev: warning: could not copy the link to the clipboard");
            }
            if let Some(mut entry) = crate::history::find_by_path(path) {
                entry.url = Some(url.clone());
                let _ = crate::history::replace(entry);
            }
            // Print just the URL so scripts can consume it.
            println!("{url}");
            Ok(Some(0))
        }
        Err(err) => {
            eprintln!("sharev: upload failed: {err}");
            Ok(Some(1))
        }
    }
}

fn run_record(kind: &str) -> Result<Option<i32>> {
    let capture_kind = match CaptureKind::parse(kind) {
        Ok(kind) => kind,
        Err(err) => {
            eprintln!("sharev: {err}");
            return Ok(Some(2));
        }
    };

    let config = Config::load_or_default();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| Error::msg(err.to_string()))?;

    eprintln!("sharev: recording {kind}; press Ctrl-C to stop");
    match runtime.block_on(crate::commands::record_until_interrupt(&config, capture_kind)) {
        Ok(result) => {
            // Print just the path so scripts can consume it.
            println!("{}", result.path);
            Ok(Some(0))
        }
        Err(Error::Cancelled) => {
            eprintln!("sharev: recording cancelled");
            Ok(Some(1))
        }
        Err(err) => {
            eprintln!("sharev: recording failed: {err}");
            Ok(Some(1))
        }
    }
}

fn print_diagnostics() {
    let config = Config::load_or_default();
    println!("ShareV {}", env!("CARGO_PKG_VERSION"));
    println!(
        "session: {} ({})",
        std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".into()),
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "unknown".into())
    );
    match crate::capture::select_backend(&config.capture_backend) {
        Ok(backend) => println!("active backend: {} ({})", backend.id(), backend.name()),
        Err(err) => println!("active backend: none ({err})"),
    }
    println!("backends:");
    for backend in crate::capture::backend_infos() {
        println!(
            "  [{}{}] {} - {}",
            if backend.available { "x" } else { " " },
            if backend.available { "" } else { " " },
            backend.id,
            backend.name
        );
    }
    println!(
        "save directory: {}",
        config.resolved_save_dir().to_string_lossy()
    );
    println!("config: {}", crate::paths::config_file().to_string_lossy());
    println!("history: {}", crate::paths::history_file().to_string_lossy());
}

fn print_help() {
    println!(
        "\
ShareV {} - ShareX-style screenshot capture for Linux

USAGE:
    sharev                       Launch the GUI
    sharev capture <kind>        Capture and print the saved path
    sharev capture-region        Shorthand for `capture region`
    sharev capture-fullscreen    Shorthand for `capture fullscreen`
    sharev capture-window        Shorthand for `capture window`
    sharev upload <path>         Upload a saved file and print the URL
    sharev record <kind>         Record until Ctrl-C, then print the path
    sharev record-region         Shorthand for `record region`
    sharev record-fullscreen     Shorthand for `record fullscreen`
    sharev record-window         Shorthand for `record window`
    sharev diagnostics           Print environment and backend information
    sharev --version             Print version

KINDS:
    region       Select a region interactively
    fullscreen   Capture/record the entire screen
    window       Capture/record the focused window",
        env!("CARGO_PKG_VERSION")
    );
}
