use async_trait::async_trait;
use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// What kind of capture the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptureKind {
    Region,
    Fullscreen,
    Window,
}

impl CaptureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CaptureKind::Region => "region",
            CaptureKind::Fullscreen => "fullscreen",
            CaptureKind::Window => "window",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "region" | "area" | "selection" => Ok(CaptureKind::Region),
            "fullscreen" | "screen" | "full" | "monitor" => Ok(CaptureKind::Fullscreen),
            "window" | "active-window" | "activewindow" => Ok(CaptureKind::Window),
            other => Err(Error::msg(format!("unknown capture kind '{other}'"))),
        }
    }
}

/// A fully specified capture request handed to a backend.
#[derive(Debug, Clone)]
pub enum CaptureRequest {
    Fullscreen,
    /// `None` means "let the user select a region interactively".
    Region { geometry: Option<Geometry> },
    /// `None` means "use the currently focused window".
    Window { id: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Geometry {
    /// Parse either slurp/grim form `"X,Y WxH"` or ImageMagick form `"WxH+X+Y"`.
    pub fn parse(input: &str) -> Result<Self> {
        let s = input.trim();
        let bad = || Error::Geometry(input.trim().to_string());

        if let Some((pos, size)) = s.split_once(' ') {
            let (x, y) = pos.split_once(',').ok_or_else(bad)?;
            let (w, h) = size.split_once('x').ok_or_else(bad)?;
            return Ok(Geometry {
                x: x.trim().parse().map_err(|_| bad())?,
                y: y.trim().parse().map_err(|_| bad())?,
                width: w.trim().parse().map_err(|_| bad())?,
                height: h.trim().parse().map_err(|_| bad())?,
            });
        }

        let (size, rest) = s.split_once('+').ok_or_else(bad)?;
        let (x, y) = rest.split_once('+').ok_or_else(bad)?;
        let (w, h) = size.split_once('x').ok_or_else(bad)?;
        Ok(Geometry {
            x: x.trim().parse().map_err(|_| bad())?,
            y: y.trim().parse().map_err(|_| bad())?,
            width: w.trim().parse().map_err(|_| bad())?,
            height: h.trim().parse().map_err(|_| bad())?,
        })
    }

    /// grim / slurp geometry string: `"X,Y WxH"`.
    pub fn to_grim(self) -> String {
        format!("{},{} {}x{}", self.x, self.y, self.width, self.height)
    }
}

/// The raw result of a capture, before it is persisted.
pub struct CapturedImage {
    pub image: RgbaImage,
    pub backend: String,
}

impl CapturedImage {
    pub fn width(&self) -> u32 {
        self.image.width()
    }

    pub fn height(&self) -> u32 {
        self.image.height()
    }
}

/// A screen capture provider.
#[async_trait]
pub trait CaptureBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    /// Whether this backend can run in the current session.
    fn is_available(&self) -> bool;
    async fn capture(&self, request: CaptureRequest) -> Result<CapturedImage>;
}

/// Decode PNG bytes (as produced by grim / import) into an RGBA image.
pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage> {
    Ok(image::load_from_memory(bytes)?.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_slurp_geometry() {
        let g = Geometry::parse("10,20 300x400").unwrap();
        assert_eq!((g.x, g.y, g.width, g.height), (10, 20, 300, 400));
    }

    #[test]
    fn parses_imagemagick_geometry() {
        let g = Geometry::parse("300x400+10+20").unwrap();
        assert_eq!((g.x, g.y, g.width, g.height), (10, 20, 300, 400));
    }

    #[test]
    fn grim_geometry_roundtrips() {
        assert_eq!(Geometry::parse("1,2 3x4").unwrap().to_grim(), "1,2 3x4");
    }

    #[test]
    fn rejects_bad_geometry() {
        assert!(Geometry::parse("nonsense").is_err());
    }

    #[test]
    fn parses_capture_kinds() {
        assert_eq!(CaptureKind::parse("Region").unwrap(), CaptureKind::Region);
        assert_eq!(
            CaptureKind::parse("fullscreen").unwrap(),
            CaptureKind::Fullscreen
        );
        assert_eq!(
            CaptureKind::parse("active-window").unwrap(),
            CaptureKind::Window
        );
        assert!(CaptureKind::parse("nope").is_err());
    }
}
