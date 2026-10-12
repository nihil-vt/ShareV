//! Scrolling capture — stitch multiple screenshots into one tall image.
//!
//! ShareV captures a fixed screen region repeatedly while the user scrolls
//! through the content. Consecutive frames overlap; [`stitch_frames`] detects
//! that overlap by comparing cheap per-row signatures and merges the frames
//! into a single tall image, and [`frame_similarity`] is used to notice when
//! the content has stopped moving so capture can finish on its own.

use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// How many columns to skip between samples when building a row signature.
const SIGNATURE_STRIDE: u32 = 8;
/// Average per-channel difference (0-255) above which an overlap candidate is
/// rejected, i.e. the frames are considered not to overlap at all.
const OVERLAP_MAX_DIFF: f32 = 22.0;

/// Settings for a scrolling capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ScrollConfig {
    /// Maximum number of frames to capture before giving up.
    pub max_frames: u32,
    /// Delay between frames in milliseconds, giving the user time to scroll.
    pub frame_delay_ms: u32,
    /// Similarity (0.0-1.0) at which two frames are considered identical and
    /// capture stops. Higher values are stricter.
    pub similarity_threshold: f32,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            max_frames: 60,
            frame_delay_ms: 450,
            similarity_threshold: 0.98,
        }
    }
}

/// Result of a scrolling capture.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollResult {
    pub output_path: String,
    pub width: u32,
    pub height: u32,
    pub frames_captured: u32,
}

/// A compact signature for each row: the RGB values of every `SIGNATURE_STRIDE`
/// column. Comparing these is far cheaper than comparing full pixels.
type RowSignatures = Vec<Vec<u8>>;

fn row_signatures(img: &RgbaImage) -> RowSignatures {
    let (width, height) = (img.width(), img.height());
    let mut signatures = Vec::with_capacity(height as usize);

    for y in 0..height {
        let mut row = Vec::with_capacity((width / SIGNATURE_STRIDE) as usize + 1);
        let mut x = 0;
        while x < width {
            let pixel = img.get_pixel(x, y);
            row.push(pixel[0]);
            row.push(pixel[1]);
            row.push(pixel[2]);
            x += SIGNATURE_STRIDE;
        }
        signatures.push(row);
    }

    signatures
}

/// Mean absolute per-channel difference between two equally long signature runs.
fn signature_diff(a: &[Vec<u8>], b: &[Vec<u8>]) -> f32 {
    let mut total = 0.0f32;
    let mut samples = 0usize;

    for (row_a, row_b) in a.iter().zip(b.iter()) {
        let len = row_a.len().min(row_b.len());
        for i in 0..len {
            total += (row_a[i] as i32 - row_b[i] as i32).abs() as f32;
            samples += 1;
        }
    }

    if samples == 0 {
        return f32::MAX;
    }
    total / samples as f32
}

/// Similarity between two frames: 1.0 means pixel-identical, 0.0 fully different.
pub fn frame_similarity(a: &DynamicImage, b: &DynamicImage) -> f32 {
    rgba_similarity(&a.to_rgba8(), &b.to_rgba8())
}

/// Similarity between two raw RGBA buffers (1.0 = identical).
pub fn rgba_similarity(a: &RgbaImage, b: &RgbaImage) -> f32 {
    let sig_a = row_signatures(a);
    let sig_b = row_signatures(b);
    let rows = sig_a.len().min(sig_b.len());

    if rows == 0 {
        return 0.0;
    }

    let diff = signature_diff(&sig_a[..rows], &sig_b[..rows]);
    1.0 - (diff / 255.0).clamp(0.0, 1.0)
}

/// Find how many rows at the bottom of `top` match the top of `bottom`.
fn find_overlap(top: &RowSignatures, bottom: &RowSignatures) -> u32 {
    let max_overlap = top.len().min(bottom.len()) as u32;
    if max_overlap == 0 {
        return 0;
    }

    let min_overlap = (max_overlap / 8).max(1);
    let mut best = 0u32;
    let mut best_score = f32::MAX;

    for overlap in min_overlap..=max_overlap {
        let n = overlap as usize;
        let score = signature_diff(&top[top.len() - n..], &bottom[..n]);
        if score < best_score {
            best_score = score;
            best = overlap;
        }
    }

    if best_score > OVERLAP_MAX_DIFF {
        0
    } else {
        best
    }
}

/// Stitch frames captured top-to-bottom into a single tall image.
///
/// Each frame is assumed to show a later portion of the same scrollable content
/// and to overlap the previous frame by some number of rows.
pub fn stitch_frames(frames: &[DynamicImage]) -> Result<DynamicImage> {
    if frames.is_empty() {
        return Err(Error::msg("no frames to stitch"));
    }
    if frames.len() == 1 {
        return Ok(frames[0].clone());
    }

    let signatures: Vec<RowSignatures> = frames
        .iter()
        .map(|f| row_signatures(&f.to_rgba8()))
        .collect();
    let width = frames.iter().map(|frame| frame.width()).min().unwrap_or(0);
    if width == 0 {
        return Err(Error::msg("captured frames have zero width"));
    }

    // Work out the overlap between each pair and the resulting total height.
    let mut overlaps = Vec::with_capacity(frames.len() - 1);
    let mut total_height = frames[0].height();
    for i in 0..frames.len() - 1 {
        let overlap = find_overlap(&signatures[i], &signatures[i + 1])
            .min(frames[i].height())
            .min(frames[i + 1].height());
        overlaps.push(overlap);
        total_height += frames[i + 1].height() - overlap;
    }

    let mut output = ImageBuffer::from_pixel(width, total_height, Rgba([0, 0, 0, 0]));

    // Copy the first frame in full.
    blit(&mut output, &frames[0].to_rgba8(), 0, 0, width);

    // Append the non-overlapping bottom part of every following frame.
    let mut y_offset = 0u32;
    for i in 0..frames.len() - 1 {
        y_offset += frames[i].height() - overlaps[i];
        blit(
            &mut output,
            &frames[i + 1].to_rgba8(),
            overlaps[i],
            y_offset,
            width,
        );
    }

    Ok(DynamicImage::ImageRgba8(output))
}

/// Copy rows `[skip..]` of `src` into `dst` starting at `dst_y`, limited to
/// `width` columns.
fn blit(
    dst: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    src: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    skip: u32,
    dst_y: u32,
    width: u32,
) {
    let dst_height = dst.height();
    for (row, y) in (skip..src.height()).enumerate() {
        let out_y = dst_y + row as u32;
        if out_y >= dst_height {
            break;
        }
        for x in 0..src.width().min(width) {
            dst.put_pixel(x, out_y, *src.get_pixel(x, y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A vertical gradient with a given row offset and height, so we can build
    /// frames with a known overlap.
    fn gradient(offset: u8, height: u32) -> DynamicImage {
        let img = ImageBuffer::from_fn(64, height, |_x, y| {
            let v = offset.wrapping_add(y as u8);
            Rgba([v, v, v, 255])
        });
        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn stitches_single_frame() {
        let img = DynamicImage::new_rgba8(100, 100);
        let result = stitch_frames(&[img]).unwrap();
        assert_eq!((result.width(), result.height()), (100, 100));
    }

    #[test]
    fn rejects_empty_frames() {
        assert!(stitch_frames(&[]).is_err());
    }

    #[test]
    fn detects_known_overlap() {
        // Frame B starts 60 rows into the content, so it overlaps A by 40 rows.
        let top = row_signatures(&gradient(0, 100).to_rgba8());
        let bottom = row_signatures(&gradient(60, 100).to_rgba8());
        assert_eq!(find_overlap(&top, &bottom), 40);
    }

    #[test]
    fn stitches_overlapping_frames() {
        let a = gradient(0, 100);
        let b = gradient(60, 100);
        let result = stitch_frames(&[a, b]).unwrap();
        assert_eq!((result.width(), result.height()), (64, 160));
    }

    #[test]
    fn identical_frames_are_similar() {
        let a = DynamicImage::new_rgba8(80, 80);
        let b = DynamicImage::new_rgba8(80, 80);
        assert!(frame_similarity(&a, &b) > 0.99);
    }

    #[test]
    fn different_frames_are_not_similar() {
        let a = gradient(0, 100);
        let b = gradient(128, 100);
        assert!(frame_similarity(&a, &b) < 0.9);
    }
}
