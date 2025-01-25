//! General converter using `image` to convert image files to WEBP

use std::io::Cursor;

use bytes::Bytes;
use image::{imageops::FilterType, DynamicImage, ImageDecoder, ImageReader};
use webp::Encoder;

use crate::{
    converter::{Converter, WebpOutput},
    error::Error,
};

pub struct General;

impl Converter for General {
    fn supported_mime_types(&self) -> &[&str] {
        &["image/jpeg", "image/png"]
    }

    fn convert(&self, data: Bytes) -> Result<WebpOutput, Error> {
        // Load image and fix orientation
        let mut decoder = ImageReader::new(Cursor::new(data))
            .with_guessed_format()?
            .into_decoder()?;
        let orientation = decoder.orientation()?;
        let mut img = DynamicImage::from_decoder(decoder)?;
        img.apply_orientation(orientation);

        // Convert image to RGB8
        let img = DynamicImage::ImageRgb8(img.into_rgb8());

        // Encode images
        let large = {
            let encoder = Encoder::from_image(&img).expect("WEBP encoding implemented for RGB");
            let webp = encoder.encode(75.0);
            Bytes::copy_from_slice(&webp)
        };
        let small = {
            let image = img.resize(600, u32::MAX, FilterType::Lanczos3);
            let encoder = Encoder::from_image(&image).expect("WEBP encoding implemented for RGB");
            let webp = encoder.encode(75.0);
            Bytes::copy_from_slice(&webp)
        };

        Ok(WebpOutput { large, small })
    }
}
