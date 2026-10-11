use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayServer {
    Wayland,
    X11,
    Unknown,
}

impl DisplayServer {
    pub fn as_str(self) -> &'static str {
        match self {
            DisplayServer::Wayland => "wayland",
            DisplayServer::X11 => "x11",
            DisplayServer::Unknown => "unknown",
        }
    }
}

/// Best-effort detection of the active display server.
pub fn detect() -> DisplayServer {
    let session = env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_lowercase();

    if session == "wayland" || env::var_os("WAYLAND_DISPLAY").is_some() {
        DisplayServer::Wayland
    } else if env::var_os("DISPLAY").is_some() {
        DisplayServer::X11
    } else {
        DisplayServer::Unknown
    }
}
