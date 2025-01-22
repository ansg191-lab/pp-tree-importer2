mod gcs;
use std::future::Future;

use bytes::Bytes;
pub use gcs::GCSBucket;
use geojson::FeatureCollection;

use crate::error::Error;

pub trait Output {
    /// Uploads a webp image to a storage location
    ///
    /// # Arguments
    ///
    /// * `id`: ID of the image
    /// * `tp`: Image Type
    /// * `data`: Image data
    fn upload_image(
        &self,
        id: &str,
        tp: ImageType,
        data: Bytes,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Upload `trees.json` to a storage location
    ///
    /// # Arguments
    ///
    /// * `json`: `trees.json` Feature Collection
    fn upload_geojson(
        &self,
        json: &FeatureCollection,
    ) -> impl Future<Output = Result<(), Error>> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    Small,
    Large,
}
