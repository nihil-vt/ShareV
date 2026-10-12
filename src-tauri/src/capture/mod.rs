pub mod backend;
pub mod portal;
pub mod wlroots;
pub mod x11;

use serde::Serialize;

pub use backend::{CaptureBackend, CaptureKind, CaptureRequest, CapturedImage, Geometry};

use crate::error::{Error, Result};

/// Locate an executable on `PATH`.
pub fn which(program: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// All known backends, in preference order for `auto`.
pub fn all_backends() -> Vec<Box<dyn CaptureBackend>> {
    vec![
        Box::new(wlroots::WlrootsBackend),
        Box::new(x11::X11Backend),
        Box::new(portal::PortalBackend),
    ]
}

/// Pick a backend by id, or the first available one for "auto".
pub fn select_backend(preferred: &str) -> Result<Box<dyn CaptureBackend>> {
    let preference = preferred.trim().to_ascii_lowercase();

    if preference.is_empty() || preference == "auto" {
        let backends = all_backends();
        let available = backends.into_iter().find(|b| b.is_available());
        return available.ok_or(Error::NoBackend);
    }

    let wanted = match preference.as_str() {
        "wlroots" | "grim" | "wayland" => "wlroots",
        "x11" | "import" | "xorg" => "x11",
        "portal" | "xdg" => "portal",
        other => return Err(Error::msg(format!("unknown capture backend '{other}'"))),
    };

    let backend = all_backends()
        .into_iter()
        .find(|b| b.id() == wanted)
        .ok_or_else(|| Error::BackendUnavailable(wanted.to_string(), "not available".into()))?;
    if !backend.is_available() {
        return Err(Error::BackendUnavailable(
            wanted.to_string(),
            "the selected backend is not available on this system".into(),
        ));
    }
    Ok(backend)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    pub id: String,
    pub name: String,
    pub available: bool,
}

pub fn backend_infos() -> Vec<BackendInfo> {
    all_backends()
        .iter()
        .map(|b| BackendInfo {
            id: b.id().to_string(),
            name: b.name().to_string(),
            available: b.is_available(),
        })
        .collect()
}
