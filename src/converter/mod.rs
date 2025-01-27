mod general;
mod heif;

use bytes::Bytes;
use valuable::{Valuable, Value, Visit};

use crate::error::Error;

pub struct ImageConverter {
    heif: heif::HeifConverter,
    general: general::General,
}

impl ImageConverter {
    pub fn new() -> Self {
        Self {
            heif: heif::HeifConverter::new(),
            general: general::General,
        }
    }

    pub fn convert(&self, format: ImageFormat, data: Bytes) -> Result<WebpOutput, Error> {
        match format {
            ImageFormat::Heif => self.heif.convert(data),
            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Webp => self.general.convert(data),
        }
    }
}

/// Converts an image to a web-friendly WEBP image
pub trait Converter: Send + Sync {
    /// Convert the image data into web-friendly WEBP image.
    fn convert(&self, data: Bytes) -> Result<WebpOutput, Error>;
}

/// WebP output images
pub struct WebpOutput {
    /// Small Image, used for popups
    /// Size: 600 x <dynamic>
    /// Quality: 75
    pub small: Bytes,
    /// Small Image, used for full screen display
    /// Size: Full Size
    /// Quality: 75
    pub large: Bytes,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ImageFormat {
    Heif,
    Jpeg,
    Png,
    Webp,
}

impl ImageFormat {
    pub fn from_mime(m: &str) -> Option<Self> {
        match m {
            "image/heif" => Some(ImageFormat::Heif),
            "image/jpeg" => Some(ImageFormat::Jpeg),
            "image/png" => Some(ImageFormat::Png),
            "image/webp" => Some(ImageFormat::Webp),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ImageFormat::Heif => "image/heif",
            ImageFormat::Jpeg => "image/jpeg",
            ImageFormat::Png => "image/png",
            ImageFormat::Webp => "image/webp",
        }
    }
}

impl Valuable for ImageFormat {
    fn as_value(&self) -> Value<'_> {
        Value::String(self.as_str())
    }
    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value())
    }
}
