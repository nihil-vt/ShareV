use async_trait::async_trait;

use super::backend::{CaptureBackend, CaptureRequest, CapturedImage};
use crate::error::{Error, Result};

/// XDG Desktop Portal capture, backed by `ashpd`.
///
/// This is the portable path for Wayland compositors that are not wlroots
/// based (GNOME, KDE). It is intentionally a stub for now: the trait and
/// selection logic already accommodate it, and it can be implemented without
/// touching the rest of the pipeline.
pub struct PortalBackend;

#[async_trait]
impl CaptureBackend for PortalBackend {
    fn id(&self) -> &'static str {
        "portal"
    }

    fn name(&self) -> &'static str {
        "XDG Desktop Portal (ashpd)"
    }

    fn is_available(&self) -> bool {
        false
    }

    async fn capture(&self, _request: CaptureRequest) -> Result<CapturedImage> {
        Err(Error::BackendUnavailable(
            "portal".into(),
            "the XDG portal backend is not implemented yet".into(),
        ))
    }
}
