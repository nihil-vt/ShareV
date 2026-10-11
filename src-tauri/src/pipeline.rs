use std::io::Cursor;

use image::{codecs::jpeg::JpegEncoder, DynamicImage, ExtendedColorType, ImageFormat as ImageCrateFormat, RgbaImage};
use serde::Serialize;

use crate::capture::{self, CaptureKind, CaptureRequest, CapturedImage, Geometry};
use crate::config::{self, Config, ImageFormat};
use crate::error::{Error, Result};
use crate::{clipboard, history};

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
}

/// Run a full capture: select backend -> grab pixels -> save -> clipboard -> history.
pub async fn perform(
    config: &Config,
    kind: CaptureKind,
    geometry: Option<Geometry>,
) -> Result<CaptureResult> {
    let backend = capture::select_backend(&config.capture_backend)?;

    let request = match kind {
        CaptureKind::Fullscreen => CaptureRequest::Fullscreen,
        CaptureKind::Region => CaptureRequest::Region { geometry },
        CaptureKind::Window => CaptureRequest::Window { id: None },
    };

    let captured = backend.capture(request).await?;
    persist(config, kind, captured).await
}

/// Persist an already-captured image according to the config.
pub async fn persist(
    config: &Config,
    kind: CaptureKind,
    captured: CapturedImage,
) -> Result<CaptureResult> {
    let format = config.image_format;
    let encoded = encode(&captured.image, format, config.jpeg_quality)?;

    let dir = config.resolved_save_dir();
    let file_name = config::build_filename(&config.filename_template, format);
    let path = config::unique_path(&dir.join(&file_name));
    std::fs::write(&path, &encoded)?;

    let id = crate::id::new_id();
    let created_at = chrono::Local::now().to_rfc3339();
    let width = captured.width();
    let height = captured.height();

    let mut copied = false;
    if config.copy_to_clipboard {
        match clipboard::copy_image(&path, format) {
            Ok(()) => copied = true,
            Err(err) => log::warn!("clipboard copy failed: {err}"),
        }
    }

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name.clone());

    let entry = history::HistoryEntry {
        id: id.clone(),
        path: path.to_string_lossy().into_owned(),
        file_name: file_name.clone(),
        kind: kind.as_str().to_string(),
        width,
        height,
        bytes: encoded.len() as u64,
        backend: captured.backend.clone(),
        created_at: created_at.clone(),
        url: None,
    };
    history::append(entry, config.max_history)?;

    Ok(CaptureResult {
        id,
        path: path.to_string_lossy().into_owned(),
        file_name,
        kind: kind.as_str().to_string(),
        width,
        height,
        bytes: encoded.len() as u64,
        backend: captured.backend,
        created_at,
        copied,
    })
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
