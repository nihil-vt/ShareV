//! Timed / event-driven auto capture.
//!
//! Grabs a fixed region (or the full screen) on an interval, optionally only
//! saving a frame when the content has actually changed, and stopping after a
//! maximum number of captures or a time limit.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::capture::CaptureKind;

/// Settings for an auto-capture session. A `max_*` of `0` means "unlimited".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AutoCaptureConfig {
    /// Milliseconds between captures.
    pub interval_ms: u64,
    /// Only save a frame when the content has changed.
    pub change_detection: bool,
    /// Similarity (0.0-1.0) at or above which two frames are considered equal.
    pub change_threshold: f32,
    /// Stop after this many saved captures (0 = unlimited).
    pub max_count: u32,
    /// Stop after this many seconds (0 = unlimited).
    pub max_duration_seconds: u64,
}

impl Default for AutoCaptureConfig {
    fn default() -> Self {
        Self {
            interval_ms: 3000,
            change_detection: false,
            change_threshold: 0.995,
            max_count: 0,
            max_duration_seconds: 0,
        }
    }
}

/// A running (or recently started) auto-capture session.
#[derive(Clone)]
pub struct AutoCaptureSession {
    /// Set to `true` to ask the capture loop to stop.
    pub stop: Arc<AtomicBool>,
    /// Wakes the capture loop early when a stop is requested.
    pub wake: Arc<Notify>,
    /// Number of captures saved so far.
    pub count: Arc<AtomicU32>,
    pub started: Instant,
    pub started_at: String,
    pub kind: CaptureKind,
    pub config: AutoCaptureConfig,
}

impl AutoCaptureSession {
    pub fn new(config: AutoCaptureConfig, kind: CaptureKind) -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            wake: Arc::new(Notify::new()),
            count: Arc::new(AtomicU32::new(0)),
            started: Instant::now(),
            started_at: chrono::Local::now().to_rfc3339(),
            kind,
            config,
        }
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        // `notify_one` stores a permit if the loop is not currently waiting, so
        // a stop requested between iterations is not lost.
        self.wake.notify_one();
    }

    pub fn is_stopping(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Wait for the next capture interval, returning early if a stop was
    /// requested. Returns `true` if the loop should stop.
    pub async fn wait_interval(&self, interval: Duration) -> bool {
        tokio::select! {
            _ = tokio::time::sleep(interval) => self.is_stopping(),
            _ = self.wake.notified() => true,
        }
    }

    pub fn info(&self) -> AutoCaptureInfo {
        AutoCaptureInfo {
            kind: self.kind.as_str().to_string(),
            interval_ms: self.config.interval_ms,
            change_detection: self.config.change_detection,
            started_at: self.started_at.clone(),
            count: self.count.load(Ordering::SeqCst),
            max_count: self.config.max_count,
            max_duration_seconds: self.config.max_duration_seconds,
        }
    }
}

/// A snapshot of an auto-capture session, sent to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoCaptureInfo {
    pub kind: String,
    pub interval_ms: u64,
    pub change_detection: bool,
    pub started_at: String,
    pub count: u32,
    pub max_count: u32,
    pub max_duration_seconds: u64,
}

/// Whether `current` differs enough from `previous` to be worth saving.
pub fn frame_changed(previous: &RgbaImage, current: &RgbaImage, threshold: f32) -> bool {
    crate::scroll::rgba_similarity(previous, current) < threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_frames_are_not_changed() {
        let a = RgbaImage::from_pixel(40, 40, image::Rgba([10, 20, 30, 255]));
        let b = a.clone();
        assert!(!frame_changed(&a, &b, 0.995));
    }

    #[test]
    fn different_frames_are_changed() {
        let a = RgbaImage::from_pixel(40, 40, image::Rgba([0, 0, 0, 255]));
        let b = RgbaImage::from_pixel(40, 40, image::Rgba([255, 255, 255, 255]));
        assert!(frame_changed(&a, &b, 0.995));
    }

    #[test]
    fn session_reports_progress() {
        let session = AutoCaptureSession::new(AutoCaptureConfig::default(), CaptureKind::Region);
        session.count.store(3, Ordering::SeqCst);
        let info = session.info();
        assert_eq!(info.count, 3);
        assert_eq!(info.kind, "region");
        session.request_stop();
        assert!(session.is_stopping());
    }

    #[tokio::test]
    async fn stop_interrupts_a_long_interval() {
        let session = AutoCaptureSession::new(
            AutoCaptureConfig {
                interval_ms: 60_000,
                ..AutoCaptureConfig::default()
            },
            CaptureKind::Region,
        );
        let stopper = session.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            stopper.request_stop();
        });

        // Should return almost immediately rather than waiting the full minute.
        assert!(session.wait_interval(Duration::from_secs(60)).await);
    }
}
