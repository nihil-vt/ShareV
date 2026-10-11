use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::capture::which;
use crate::config::ImageFormat;
use crate::error::{Error, Result};
use crate::platform::display::{detect, DisplayServer};

/// Copy an image file to the system clipboard.
///
/// Uses `wl-copy` on Wayland and `xclip` on X11 — both are ubiquitous on
/// Linux desktops and avoid pulling a large clipboard dependency.
pub fn copy_image(path: &Path, format: ImageFormat) -> Result<()> {
    let data = std::fs::read(path)?;
    copy_bytes(&data, format.mime())
}

/// Copy raw image bytes to the system clipboard with an explicit mime type.
pub fn copy_bytes(data: &[u8], mime: &str) -> Result<()> {
    match detect() {
        DisplayServer::Wayland if which("wl-copy").is_some() => {
            pipe_to("wl-copy", &["--type", mime], data)
        }
        _ if which("xclip").is_some() => {
            pipe_to("xclip", &["-selection", "clipboard", "-t", mime, "-i"], data)
        }
        _ => Err(Error::msg(
            "no clipboard tool available (install wl-clipboard or xclip)",
        )),
    }
}

/// Copy plain text to the system clipboard.
pub fn copy_text(text: &str) -> Result<()> {
    match detect() {
        DisplayServer::Wayland if which("wl-copy").is_some() => {
            pipe_to("wl-copy", &[], text.as_bytes())
        }
        _ if which("xclip").is_some() => {
            pipe_to("xclip", &["-selection", "clipboard", "-i"], text.as_bytes())
        }
        _ => Err(Error::msg(
            "no clipboard tool available (install wl-clipboard or xclip)",
        )),
    }
}

fn pipe_to(program: &str, args: &[&str], data: &[u8]) -> Result<()> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        // Do NOT pipe stderr/stdout: clipboard helpers fork a background
        // process that inherits these fds, so reading them to EOF would block.
        .stderr(Stdio::null())
        .spawn()
        .map_err(Error::Io)?;

    if let Some(mut stdin) = child.stdin.take() {
        // Ignore a broken pipe: the helper may close stdin once it is done.
        let _ = stdin.write_all(data);
        // `stdin` is dropped here, signalling EOF.
    }

    // Wait for the foreground process, but never block forever: some helpers
    // keep a process alive to serve the selection.
    let deadline = Instant::now() + Duration::from_millis(1500);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return if status.success() {
                    Ok(())
                } else {
                    Err(Error::msg(format!("{program} exited with status {status}")))
                };
            }
            Ok(None) if Instant::now() >= deadline => {
                // Still running — assume it is serving the selection. Detach a
                // reaper so it does not become a zombie.
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(err) => return Err(Error::Io(err)),
        }
    }
}
