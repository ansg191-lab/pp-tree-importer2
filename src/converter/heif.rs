//! Convert using `libheif` to convert HEIF/HEIC files to WEBP

use bytes::Bytes;
use image::{imageops::FilterType, DynamicImage, RgbImage};
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma};
use webp::Encoder;

use crate::{
    converter::{Converter, WebpOutput},
    error::Error,
};

pub struct HeifConverter {
    heif: LibHeif,
}

impl HeifConverter {
    pub fn new() -> Self {
        Self {
            heif: LibHeif::new(),
        }
    }
}

impl Converter for HeifConverter {
    fn supported_mime_types(&self) -> &[&str] {
        &["image/heif", "image/heic"]
    }

    #[tracing::instrument(skip_all)]
    fn convert(&self, data: Bytes) -> Result<WebpOutput, Error> {
        let ctx = HeifContext::read_from_bytes(&data)?;
        let primary = ctx.primary_image_handle()?;

        // Decode image
        let image = self
            .heif
            .decode(&primary, ColorSpace::Rgb(RgbChroma::Rgb), None)?;
        let width = image.width();
        let height = image.height();
        let planes = image.planes();
        let interleaved = planes.interleaved.ok_or(Error::LibHeifMissingInterleaved)?;

        // Create `image` Image
        let rgba = RgbImage::from_raw(width, height, interleaved.data.to_vec()).ok_or(
            Error::LibHeifDataLengthMismatch {
                width: width as usize,
                height: height as usize,
                length: interleaved.data.len(),
            },
        )?;
        let image = DynamicImage::ImageRgb8(rgba);

        let large = {
            let encoder = Encoder::from_image(&image).expect("WEBP encoding implemented for RGB");
            let webp = encoder.encode(75.0);
            Bytes::copy_from_slice(&*webp)
        };
        let small = {
            let image = image.resize(600, u32::MAX, FilterType::Lanczos3);
            let encoder = Encoder::from_image(&image).expect("WEBP encoding implemented for RGB");
            let webp = encoder.encode(75.0);
            Bytes::copy_from_slice(&*webp)
        };

        Ok(WebpOutput { small, large })
    }
}
