//! Image effects pipeline.
//!
//! Post-capture effects like ShareX: shadow, border, watermark, canvas
//! resize, blur, and rounded corners. Effects are applied in sequence
//! to an RGBA image.

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, imageops};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// A single image effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    #[default]
    None,
    Shadow,
    Border,
    Watermark,
    CanvasResize,
    Blur,
    RoundedCorners,
}

impl EffectKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EffectKind::None => "none",
            EffectKind::Shadow => "shadow",
            EffectKind::Border => "border",
            EffectKind::Watermark => "watermark",
            EffectKind::CanvasResize => "canvasResize",
            EffectKind::Blur => "blur",
            EffectKind::RoundedCorners => "roundedCorners",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "shadow" => Some(EffectKind::Shadow),
            "border" => Some(EffectKind::Border),
            "watermark" => Some(EffectKind::Watermark),
            "canvasresize" | "resize" => Some(EffectKind::CanvasResize),
            "blur" => Some(EffectKind::Blur),
            "roundedcorners" | "rounded" => Some(EffectKind::RoundedCorners),
            _ => None,
        }
    }
}

/// Shadow effect parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ShadowParams {
    pub offset_x: i32,
    pub offset_y: i32,
    pub blur_radius: f32,
    pub color: String, // hex, e.g. "#000000"
    pub opacity: f32,  // 0.0 - 1.0
}

impl Default for ShadowParams {
    fn default() -> Self {
        Self {
            offset_x: 4,
            offset_y: 4,
            blur_radius: 8.0,
            color: "#000000".into(),
            opacity: 0.5,
        }
    }
}

/// Border effect parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BorderParams {
    pub width: u32,
    pub color: String,
}

impl Default for BorderParams {
    fn default() -> Self {
        Self {
            width: 2,
            color: "#ffffff".into(),
        }
    }
}

/// Watermark effect parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WatermarkParams {
    pub text: String,
    pub font_size: u32,
    pub color: String,
    pub opacity: f32,
    pub position: String, // "top-left", "top-right", "bottom-left", "bottom-right", "center"
}

impl Default for WatermarkParams {
    fn default() -> Self {
        Self {
            text: "ShareV".into(),
            font_size: 24,
            color: "#ffffff".into(),
            opacity: 0.5,
            position: "bottom-right".into(),
        }
    }
}

/// Canvas resize effect parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CanvasResizeParams {
    pub padding: u32,
    pub color: String,
}

impl Default for CanvasResizeParams {
    fn default() -> Self {
        Self {
            padding: 20,
            color: "#ffffff".into(),
        }
    }
}

/// Blur effect parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BlurParams {
    pub radius: f32,
}

impl Default for BlurParams {
    fn default() -> Self {
        Self { radius: 5.0 }
    }
}

/// Rounded corners effect parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RoundedCornersParams {
    pub radius: u32,
}

impl Default for RoundedCornersParams {
    fn default() -> Self {
        Self { radius: 16 }
    }
}

/// Ordered list of effects to apply, with their parameters.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct EffectsConfig {
    pub effects: Vec<EffectKind>,
    pub shadow: ShadowParams,
    pub border: BorderParams,
    pub watermark: WatermarkParams,
    pub canvas_resize: CanvasResizeParams,
    pub blur: BlurParams,
    pub rounded_corners: RoundedCornersParams,
}

fn parse_hex_color(hex: &str) -> Result<[u8; 4]> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| Error::msg("invalid hex color"))?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| Error::msg("invalid hex color"))?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| Error::msg("invalid hex color"))?;
        Ok([r, g, b, 255])
    } else if hex.len() == 8 {
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| Error::msg("invalid hex color"))?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| Error::msg("invalid hex color"))?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| Error::msg("invalid hex color"))?;
        let a = u8::from_str_radix(&hex[6..8], 16).map_err(|_| Error::msg("invalid hex color"))?;
        Ok([r, g, b, a])
    } else {
        Err(Error::msg("hex color must be #RRGGBB or #RRGGBBAA"))
    }
}

/// Apply the configured effects to an image in sequence.
pub fn apply_effects(image: &DynamicImage, config: &EffectsConfig) -> DynamicImage {
    let mut img = image.clone();

    for effect in &config.effects {
        match effect {
            EffectKind::None => {}
            EffectKind::Shadow => img = apply_shadow(&img, &config.shadow),
            EffectKind::Border => img = apply_border(&img, &config.border),
            EffectKind::Watermark => img = apply_watermark(&img, &config.watermark),
            EffectKind::CanvasResize => img = apply_canvas_resize(&img, &config.canvas_resize),
            EffectKind::Blur => img = apply_blur_effect(&img, &config.blur),
            EffectKind::RoundedCorners => {
                img = apply_rounded_corners(&img, &config.rounded_corners)
            }
        }
    }

    img
}

fn apply_shadow(image: &DynamicImage, params: &ShadowParams) -> DynamicImage {
    let color = parse_hex_color(&params.color).unwrap_or([0, 0, 0, 255]);
    let alpha = (params.opacity.clamp(0.0, 1.0) * 255.0) as u8;
    let shadow_color = [color[0], color[1], color[2], alpha];

    let (w, h) = image.dimensions();
    let blur = params.blur_radius.max(0.0);
    // Pad symmetrically so an offset in any direction (including negative) and
    // the blur spread both fit inside the canvas.
    let pad_x = blur + params.offset_x.unsigned_abs() as f32;
    let pad_y = blur + params.offset_y.unsigned_abs() as f32;
    let base_x = pad_x.round() as i32;
    let base_y = pad_y.round() as i32;
    let new_w = w + (pad_x.round() as u32) * 2;
    let new_h = h + (pad_y.round() as u32) * 2;

    let mut canvas = ImageBuffer::from_pixel(new_w, new_h, Rgba([0, 0, 0, 0]));

    // Draw a solid silhouette of the image, shifted by the configured offset.
    let shadow_img = image.to_rgba8();
    for y in 0..shadow_img.height() {
        for x in 0..shadow_img.width() {
            if shadow_img.get_pixel(x, y)[3] == 0 {
                continue;
            }
            let nx = x as i32 + base_x + params.offset_x;
            let ny = y as i32 + base_y + params.offset_y;
            if nx >= 0 && ny >= 0 && (nx as u32) < new_w && (ny as u32) < new_h {
                canvas.put_pixel(nx as u32, ny as u32, Rgba(shadow_color));
            }
        }
    }

    // Blur the silhouette, then composite the original on top.
    let mut result = if blur > 0.0 {
        DynamicImage::ImageRgba8(canvas).blur(blur)
    } else {
        DynamicImage::ImageRgba8(canvas)
    };
    imageops::overlay(&mut result, image, base_x as i64, base_y as i64);

    result
}

fn apply_border(image: &DynamicImage, params: &BorderParams) -> DynamicImage {
    let color = parse_hex_color(&params.color).unwrap_or([255, 255, 255, 255]);
    let (w, h) = image.dimensions();
    let b = params.width;
    let new_w = w + b * 2;
    let new_h = h + b * 2;

    let mut canvas = ImageBuffer::from_pixel(new_w, new_h, Rgba(color));
    imageops::overlay(&mut canvas, &image.to_rgba8(), b as i64, b as i64);

    DynamicImage::ImageRgba8(canvas)
}

fn apply_watermark(image: &DynamicImage, params: &WatermarkParams) -> DynamicImage {
    let color = parse_hex_color(&params.color).unwrap_or([255, 255, 255, 255]);
    let alpha = (params.opacity * 255.0) as u8;
    let text_color = [color[0], color[1], color[2], alpha];

    let (w, h) = image.dimensions();
    let mut canvas = image.to_rgba8();

    // Simple text rendering: draw the watermark text as a series of rectangles
    // For a full implementation, you'd use a font renderer like rusttype
    let text = &params.text;
    let font_size = params.font_size.max(8);
    let char_width = font_size / 2;
    let text_width = (text.len() as u32) * char_width;
    let text_height = font_size;

    let (tx, ty) = match params.position.as_str() {
        "top-left" => (10, 10),
        "top-right" => (w.saturating_sub(text_width + 10), 10),
        "bottom-left" => (10, h.saturating_sub(text_height + 10)),
        "bottom-right" => (
            w.saturating_sub(text_width + 10),
            h.saturating_sub(text_height + 10),
        ),
        "center" => (
            (w.saturating_sub(text_width)) / 2,
            (h.saturating_sub(text_height)) / 2,
        ),
        _ => (
            w.saturating_sub(text_width + 10),
            h.saturating_sub(text_height + 10),
        ),
    };

    // Draw text background (semi-transparent)
    for y in ty.saturating_sub(2)..(ty + text_height + 2).min(h) {
        for x in tx.saturating_sub(2)..(tx + text_width + 2).min(w) {
            let pixel = canvas.get_pixel(x, y);
            let blended = blend_pixel(*pixel, Rgba([0, 0, 0, 128]));
            canvas.put_pixel(x, y, blended);
        }
    }

    // Draw simple text representation (blocky but visible)
    for (i, _ch) in text.chars().enumerate() {
        let cx = tx + (i as u32) * char_width;
        for dy in 0..text_height {
            for dx in 0..char_width.saturating_sub(1) {
                let px = cx + dx;
                let py = ty + dy;
                if px < w && py < h {
                    // Simple block pattern for each character
                    if (dx + dy) % 3 != 0 {
                        canvas.put_pixel(px, py, Rgba(text_color));
                    }
                }
            }
        }
    }

    DynamicImage::ImageRgba8(canvas)
}

fn apply_canvas_resize(image: &DynamicImage, params: &CanvasResizeParams) -> DynamicImage {
    let color = parse_hex_color(&params.color).unwrap_or([255, 255, 255, 255]);
    let (w, h) = image.dimensions();
    let p = params.padding;
    let new_w = w + p * 2;
    let new_h = h + p * 2;

    let mut canvas = ImageBuffer::from_pixel(new_w, new_h, Rgba(color));
    imageops::overlay(&mut canvas, &image.to_rgba8(), p as i64, p as i64);

    DynamicImage::ImageRgba8(canvas)
}

fn apply_blur_effect(image: &DynamicImage, params: &BlurParams) -> DynamicImage {
    if params.radius <= 0.0 {
        return image.clone();
    }
    image.blur(params.radius)
}

fn apply_rounded_corners(image: &DynamicImage, params: &RoundedCornersParams) -> DynamicImage {
    let radius = params.radius.min(image.width().min(image.height()) / 2);
    if radius == 0 {
        return image.clone();
    }

    let (w, h) = image.dimensions();
    let mut canvas = ImageBuffer::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    let src = image.to_rgba8();

    for y in 0..h {
        for x in 0..w {
            let pixel = src.get_pixel(x, y);
            if pixel[3] == 0 {
                continue;
            }

            // Check if pixel is in a corner region
            let in_left = x < radius;
            let in_right = x >= w - radius;
            let in_top = y < radius;
            let in_bottom = y >= h - radius;

            let transparent = if in_left && in_top {
                let dx = radius - x;
                let dy = radius - y;
                (dx * dx + dy * dy) > radius * radius
            } else if in_right && in_top {
                let dx = x - (w - radius - 1);
                let dy = radius - y;
                (dx * dx + dy * dy) > radius * radius
            } else if in_left && in_bottom {
                let dx = radius - x;
                let dy = y - (h - radius - 1);
                (dx * dx + dy * dy) > radius * radius
            } else if in_right && in_bottom {
                let dx = x - (w - radius - 1);
                let dy = y - (h - radius - 1);
                (dx * dx + dy * dy) > radius * radius
            } else {
                false
            };

            if !transparent {
                canvas.put_pixel(x, y, *pixel);
            }
        }
    }

    DynamicImage::ImageRgba8(canvas)
}

fn blend_pixel(bottom: Rgba<u8>, top: Rgba<u8>) -> Rgba<u8> {
    let a = top[3] as f32 / 255.0;
    let r = (top[0] as f32 * a + bottom[0] as f32 * (1.0 - a)) as u8;
    let g = (top[1] as f32 * a + bottom[1] as f32 * (1.0 - a)) as u8;
    let b = (top[2] as f32 * a + bottom[2] as f32 * (1.0 - a)) as u8;
    Rgba([r, g, b, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_hex_color("#ffffff").unwrap(), [255, 255, 255, 255]);
        assert_eq!(parse_hex_color("#FF0000").unwrap(), [255, 0, 0, 255]);
        assert_eq!(parse_hex_color("#FF000080").unwrap(), [255, 0, 0, 128]);
    }

    #[test]
    fn rejects_bad_hex() {
        assert!(parse_hex_color("#fff").is_err());
        assert!(parse_hex_color("#gggggg").is_err());
    }

    #[test]
    fn applies_no_effects() {
        let img = DynamicImage::new_rgba8(100, 100);
        let config = EffectsConfig::default();
        let result = apply_effects(&img, &config);
        assert_eq!(result.dimensions(), (100, 100));
    }

    #[test]
    fn applies_border() {
        let img = DynamicImage::new_rgba8(100, 100);
        let config = EffectsConfig {
            effects: vec![EffectKind::Border],
            border: BorderParams {
                width: 5,
                color: "#ff0000".into(),
            },
            ..Default::default()
        };
        let result = apply_effects(&img, &config);
        assert_eq!(result.dimensions(), (110, 110));
    }

    #[test]
    fn applies_canvas_resize() {
        let img = DynamicImage::new_rgba8(100, 100);
        let config = EffectsConfig {
            effects: vec![EffectKind::CanvasResize],
            canvas_resize: CanvasResizeParams {
                padding: 20,
                color: "#000000".into(),
            },
            ..Default::default()
        };
        let result = apply_effects(&img, &config);
        assert_eq!(result.dimensions(), (140, 140));
    }

    #[test]
    fn shadow_pads_symmetrically() {
        let img = DynamicImage::new_rgba8(100, 100);
        let config = EffectsConfig {
            effects: vec![EffectKind::Shadow],
            shadow: ShadowParams {
                offset_x: -6,
                offset_y: 6,
                blur_radius: 4.0,
                color: "#000000".into(),
                opacity: 0.5,
            },
            ..Default::default()
        };
        let result = apply_effects(&img, &config);
        // pad = blur(4) + max(|offset|) (6) = 10 per side.
        assert_eq!(result.dimensions(), (120, 120));
    }

    #[test]
    fn zero_radius_blur_is_a_noop() {
        let img = DynamicImage::new_rgba8(20, 20);
        let config = EffectsConfig {
            effects: vec![EffectKind::Blur],
            blur: BlurParams { radius: 0.0 },
            ..Default::default()
        };
        let result = apply_effects(&img, &config);
        assert_eq!(result.dimensions(), (20, 20));
    }
}
