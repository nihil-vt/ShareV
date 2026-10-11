use std::io::Cursor;

use image::{codecs::jpeg::JpegEncoder, DynamicImage, ExtendedColorType, ImageFormat as ImageCrateFormat, RgbaImage};
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
