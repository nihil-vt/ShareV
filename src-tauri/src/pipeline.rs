use std::io::Cursor;
use std::time::Duration;

use image::{
    DynamicImage, ExtendedColorType, ImageFormat as ImageCrateFormat, RgbaImage,
    codecs::jpeg::JpegEncoder,
};
use serde::Serialize;

use crate::capture::{self, CaptureKind, CaptureRequest, Geometry};
use crate::config::{self, Config, ImageFormat};
use crate::error::{Error, Result};

/// Everything the frontend needs to know about a finished capture.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub id: String,
    pub path: String,
    pub file_name: String,
    pub kind: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub backend: String,
    pub created_at: String,
    pub copied: bool,
    /// URL returned by the `upload` task, if it ran successfully.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Whether the frontend should open the annotation editor.
    pub open_editor: bool,
}

/// A capture that has been grabbed and encoded, but not yet acted on.
pub struct CaptureDraft {
    pub image: RgbaImage,
    pub encoded: Vec<u8>,
    pub format: ImageFormat,
    pub kind: CaptureKind,
    pub backend: String,
    /// Screen region that was captured, when known.
    pub geometry: Option<Geometry>,
}

/// Grab pixels from a backend and encode them, without saving or running tasks.
pub async fn grab(
    config: &Config,
    kind: CaptureKind,
    geometry: Option<Geometry>,
    monitor: Option<String>,
    window_id: Option<String>,
) -> Result<CaptureDraft> {
    let backend = capture::select_backend(&config.capture_backend)?;

    let request = match kind {
        CaptureKind::Fullscreen => {
            let monitor = monitor.or_else(|| {
                let configured = config.monitor.trim();
                (!configured.is_empty()).then(|| configured.to_string())
            });
            CaptureRequest::Fullscreen { monitor }
        }
        CaptureKind::Region => CaptureRequest::Region { geometry },
        CaptureKind::Window => CaptureRequest::Window { id: window_id },
        // A single grab of a scrolling capture behaves like a region capture;
        // `grab_scrolling` performs the actual multi-frame stitching.
        CaptureKind::Scrolling => CaptureRequest::Region { geometry },
    };

    let captured = backend.capture(request).await?;
    let encoded = encode(&captured.image, config.image_format, config.jpeg_quality)?;

    Ok(CaptureDraft {
        image: captured.image,
        encoded,
        format: config.image_format,
        kind,
        backend: captured.backend,
        geometry: captured.geometry,
    })
}

/// Capture a region repeatedly while the user scrolls, then stitch the frames
/// into a single tall image.
///
/// The initial region selection also provides the first frame. `on_selected`
/// is invoked with the chosen geometry so callers can prompt the user to
/// start scrolling (e.g. with a desktop notification).
pub async fn grab_scrolling<F>(config: &Config, on_selected: F) -> Result<CaptureDraft>
where
    F: FnOnce(Geometry),
{
    let backend = capture::select_backend(&config.capture_backend)?;
    let scroll = &config.scroll;

    let first = backend
        .capture(CaptureRequest::Region { geometry: None })
        .await?;
    let geometry = first
        .geometry
        .ok_or_else(|| Error::msg("could not determine the selected region"))?;
    on_selected(geometry);

    let mut frames: Vec<DynamicImage> = vec![DynamicImage::ImageRgba8(first.image.clone())];
    let mut previous = first.image.clone();

    for _ in 1..scroll.max_frames.max(1) {
        tokio::time::sleep(Duration::from_millis(scroll.frame_delay_ms.max(1) as u64)).await;
        let next = backend
            .capture(CaptureRequest::Region {
                geometry: Some(geometry),
            })
            .await?;
        let frame = DynamicImage::ImageRgba8(next.image.clone());
        if crate::scroll::frame_similarity(&DynamicImage::ImageRgba8(previous), &frame)
            >= scroll.similarity_threshold.clamp(0.0, 1.0)
        {
            break;
        }
        previous = next.image;
        frames.push(frame);
    }

    let stitched = crate::scroll::stitch_frames(&frames)?;
    let image = stitched.to_rgba8();
    let encoded = encode(&image, config.image_format, config.jpeg_quality)?;

    Ok(CaptureDraft {
        image,
        encoded,
        format: config.image_format,
        kind: CaptureKind::Scrolling,
        backend: first.backend,
        geometry: Some(geometry),
    })
}

/// Capture a fixed region (or the full screen) without any interaction.
///
/// Used by auto capture and any other repeated grab: when `geometry` is `Some`
/// the same region is re-captured each time; otherwise the full screen (or the
/// given monitor) is captured.
pub async fn grab_locked(
    config: &Config,
    geometry: Option<Geometry>,
    monitor: Option<String>,
) -> Result<CaptureDraft> {
    let backend = capture::select_backend(&config.capture_backend)?;
    let request = match geometry {
        Some(geometry) => CaptureRequest::Region {
            geometry: Some(geometry),
        },
        None => CaptureRequest::Fullscreen { monitor },
    };

    let captured = backend.capture(request).await?;
    let encoded = encode(&captured.image, config.image_format, config.jpeg_quality)?;

    Ok(CaptureDraft {
        image: captured.image,
        encoded,
        format: config.image_format,
        kind: if geometry.is_some() {
            CaptureKind::Region
        } else {
            CaptureKind::Fullscreen
        },
        backend: captured.backend,
        geometry: captured.geometry,
    })
}

/// Run a full capture with the default (non-interactive) parameters. Convenience
/// wrapper used by the headless CLI.
pub async fn perform(
    config: &Config,
    kind: CaptureKind,
    geometry: Option<Geometry>,
) -> Result<CaptureResult> {
    let draft = grab(config, kind, geometry, None, None).await?;
    crate::tasks::run(None, config, draft).await
}

/// Encode an RGBA image into the configured format.
pub fn encode(image: &RgbaImage, format: ImageFormat, quality: u8) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    let dynamic = DynamicImage::ImageRgba8(image.clone());

    match format {
        ImageFormat::Png => {
            dynamic.write_to(&mut Cursor::new(&mut buffer), ImageCrateFormat::Png)?;
        }
        ImageFormat::Webp => {
            dynamic.write_to(&mut Cursor::new(&mut buffer), ImageCrateFormat::WebP)?;
        }
        ImageFormat::Jpeg => {
            let rgb = dynamic.to_rgb8();
            let mut encoder =
                JpegEncoder::new_with_quality(&mut buffer, config::clamp_quality(quality));
            encoder.encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                ExtendedColorType::Rgb8,
            )?;
        }
    }

    if buffer.is_empty() {
        return Err(Error::EmptyCapture);
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_supported_formats() {
        let image = RgbaImage::from_pixel(4, 4, image::Rgba([10, 20, 30, 255]));
        for format in [ImageFormat::Png, ImageFormat::Jpeg, ImageFormat::Webp] {
            let bytes = encode(&image, format, 80).unwrap();
            assert!(!bytes.is_empty(), "{format:?} produced no bytes");
        }
    }

    #[test]
    fn encoded_png_is_a_png() {
        let image = RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]));
        let bytes = encode(&image, ImageFormat::Png, 90).unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }
}
