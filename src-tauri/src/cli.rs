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
//!
//! Common flags: `--delay <seconds>`, `--monitor <name>` and `--window <id>`.

use std::time::Duration;

use crate::capture::CaptureKind;
use crate::config::Config;
use crate::error::{Error, Result};

/// Parsed positional capture/record arguments and flags.
#[derive(Debug, Default)]
struct CommandArgs {
    kind: Option<String>,
    delay: u32,
    monitor: String,
    window_id: Option<String>,
}

impl CommandArgs {
    fn parse(args: &[String]) -> std::result::Result<Self, String> {
        let mut parsed = CommandArgs::default();
        let mut iter = args.iter().peekable();
        while let Some(arg) = iter.next() {
            let (flag, inline) = match arg.split_once('=') {
                Some((flag, value)) => (flag, Some(value.to_string())),
                None => (arg.as_str(), None),
            };
            let take_value = |iter: &mut std::iter::Peekable<std::slice::Iter<'_, String>>| {
                inline.clone().or_else(|| iter.next().cloned())
            };
            match flag {
                "--delay" | "-d" => {
                    let value = take_value(&mut iter)
                        .ok_or_else(|| "--delay needs a value".to_string())?;
                    parsed.delay = value
                        .parse()
                        .map_err(|_| format!("invalid delay '{value}'"))?;
                }
                "--monitor" | "-m" => {
                    parsed.monitor = take_value(&mut iter)
                        .ok_or_else(|| "--monitor needs a value".to_string())?;
                }
                "--window" | "-w" => {
                    parsed.window_id = Some(
                        take_value(&mut iter)
                            .ok_or_else(|| "--window needs a value".to_string())?,
                    );
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option '{other}'"));
                }
                _ => {
                    if parsed.kind.is_none() {
                        parsed.kind = Some(arg.clone());
                    }
                }
            }
        }
        Ok(parsed)
    }
}

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
        "capture" => run_capture(&args[1..]),
        "capture-region" => run_capture(&prepend("region", &args[1..])),
        "capture-fullscreen" => run_capture(&prepend("fullscreen", &args[1..])),
        "capture-window" => run_capture(&prepend("window", &args[1..])),
        "upload" => run_upload(args.get(1).map(String::as_str)),
        "record" => run_record(&args[1..]),
        "record-region" => run_record(&prepend("region", &args[1..])),
        "record-fullscreen" => run_record(&prepend("fullscreen", &args[1..])),
        "record-window" => run_record(&prepend("window", &args[1..])),
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

fn prepend(value: &str, rest: &[String]) -> Vec<String> {
    let mut args = Vec::with_capacity(rest.len() + 1);
    args.push(value.to_string());
    args.extend_from_slice(rest);
    args
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| Error::msg(err.to_string()))
}

fn run_capture(args: &[String]) -> Result<Option<i32>> {
    let parsed = match CommandArgs::parse(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("sharev: {err}");
            return Ok(Some(2));
        }
    };
    let kind = parsed.kind.as_deref().unwrap_or("region");
    let capture_kind = match CaptureKind::parse(kind) {
        Ok(kind) => kind,
        Err(err) => {
            eprintln!("sharev: {err}");
            return Ok(Some(2));
        }
    };

    let config = Config::load_or_default();
    let runtime = runtime()?;
    let monitor = (!parsed.monitor.trim().is_empty()).then(|| parsed.monitor.clone());

    let result = runtime.block_on(async {
        if parsed.delay > 0 {
            tokio::time::sleep(Duration::from_secs(parsed.delay as u64)).await;
        }
        let draft = crate::pipeline::grab(
            &config,
            capture_kind,
            None,
            monitor,
            parsed.window_id.clone(),
        )
        .await?;
        crate::tasks::run(None, &config, draft).await
    });

    match result {
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
    let runtime = runtime()?;

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

fn run_record(args: &[String]) -> Result<Option<i32>> {
    let parsed = match CommandArgs::parse(args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("sharev: {err}");
            return Ok(Some(2));
        }
    };
    let kind = parsed.kind.as_deref().unwrap_or("region");
    let capture_kind = match CaptureKind::parse(kind) {
        Ok(kind) => kind,
        Err(err) => {
            eprintln!("sharev: {err}");
            return Ok(Some(2));
        }
    };

    let config = Config::load_or_default();
    let runtime = runtime()?;
    let monitor = if parsed.monitor.trim().is_empty() {
        config.monitor.clone()
    } else {
        parsed.monitor.clone()
    };

    eprintln!("sharev: recording {kind}; press Ctrl-C to stop");
    match runtime.block_on(crate::commands::record_until_interrupt(
        &config,
        capture_kind,
        parsed.delay,
        monitor,
        parsed.window_id.clone(),
    )) {
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

OPTIONS:
    -d, --delay <seconds>        Wait before capturing / recording
    -m, --monitor <name>         Target a specific monitor
    -w, --window <id>            Target a specific window

KINDS:
    region       Select a region interactively
    fullscreen   Capture/record the entire screen
    window       Capture/record the focused window",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_positional_kind_and_flags() {
        let parsed = CommandArgs::parse(&args(&["window", "--delay", "3", "--monitor", "HDMI-1"]))
            .unwrap();
        assert_eq!(parsed.kind.as_deref(), Some("window"));
        assert_eq!(parsed.delay, 3);
        assert_eq!(parsed.monitor, "HDMI-1");
    }

    #[test]
    fn parses_inline_flag_values() {
        let parsed = CommandArgs::parse(&args(&["--delay=5", "--window=0x123"])).unwrap();
        assert_eq!(parsed.delay, 5);
        assert_eq!(parsed.window_id.as_deref(), Some("0x123"));
        assert!(parsed.kind.is_none());
    }

    #[test]
    fn rejects_unknown_options_and_bad_delay() {
        assert!(CommandArgs::parse(&args(&["--nope"])).is_err());
        assert!(CommandArgs::parse(&args(&["--delay", "abc"])).is_err());
    }
}
