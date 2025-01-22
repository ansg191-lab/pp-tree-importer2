mod general;
mod heif;

use std::sync::Arc;

use bytes::Bytes;

use crate::error::Error;

/// Converts an image to a web-friendly WEBP image
pub trait Converter: Send + Sync {
    /// Supported mime-types that this converter can process.
    fn supported_mime_types(&self) -> &[&str];

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

pub fn converters() -> Arc<[Box<dyn Converter>]> {
    Arc::new([
        Box::new(heif::HeifConverter::new()),
        Box::new(general::General),
    ])
}
