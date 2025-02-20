//! General converter using `image` to convert image files to WEBP

use std::io::Cursor;

use bytes::Bytes;
use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType};
use webp::Encoder;

use crate::{
    converter::{Converter, WebpOutput},
    error::Error,
};

pub struct General;

impl Converter for General {
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

#[cfg(test)]
mod tests {
    use image::ImageFormat;

    use super::*;

    #[test]
    fn convert() {
        let file = std::fs::read("fixtures/20250121_065541.jpg").unwrap();
        let bytes = Bytes::from(file);

        let converter = General;
        let output = converter.convert(bytes.clone()).unwrap();

        assert!(
            output.small.len() < output.large.len(),
            "Small image should be smaller than large image"
        );
        assert!(
            output.small.len() < bytes.len(),
            "Small image should be smaller than original image"
        );
        assert!(
            output.large.len() < bytes.len(),
            "Large image should be smaller than original image"
        );

        let small = image::load_from_memory_with_format(&output.small, ImageFormat::WebP).unwrap();
        assert_eq!(small.width(), 600);
        assert_eq!(small.height(), 1333);
        assert_eq!(small.color(), image::ColorType::Rgb8);

        let large = image::load_from_memory_with_format(&output.large, ImageFormat::WebP).unwrap();
        assert_eq!(large.width(), 1800);
        assert_eq!(large.height(), 4000);
        assert_eq!(large.color(), image::ColorType::Rgb8);
    }
}
