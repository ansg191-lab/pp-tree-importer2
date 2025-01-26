use std::sync::Arc;

use futures::StreamExt;
use geojson::{Feature, FeatureCollection};
use peak_alloc::PeakAlloc;
use tokio::time::Instant;
use tracing::{debug, error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use valuable::Valuable;

use crate::{
    config::Config,
    converter::Converter,
    error::Error,
    image_source::{GDrive, Image, ImageSource},
    metadata::Tree,
    output::{GCSBucket, ImageType, Output},
};

mod config;
mod converter;
mod error;
mod http;
mod image_source;
mod macros;
mod metadata;
mod output;

const ALLOWED_MIME_TYPES: [&str; 3] = ["image/heif", "image/jpeg", "image/png"];

#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::from_env()?;

    tracing_subscriber::registry()
        .with(config.log_format.into_layer())
        .with(EnvFilter::from_default_env())
        .init();

    let now = Instant::now();
    info!(config = config.as_value(), "Starting sync");

    let gdrive = GDrive::new(Arc::clone(&config)).await?;
    let converters = converter::converters();
    let output = GCSBucket::new(Arc::clone(&config)).await?;

    // Run download and processing
    let trees = gdrive
        .images()
        .map(|res| process_image(&gdrive, Arc::clone(&converters), &output, res))
        .buffer_unordered(config.concurrency)
        .filter_map(|x| async move { x })
        .collect::<Vec<Tree>>()
        .await;

    // Convert trees to features
    let features = trees
        .into_iter()
        .map(Feature::from)
        .collect::<Vec<Feature>>();
    let collection = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };

    // Upload geojson to output
    info!("Uploading geojson to output");
    output.upload_geojson(&collection).await?;

    info!(
        total_trees = collection.features.len(),
        peak_mem = PEAK_ALLOC.peak_usage(),
        peak_mem_mb = PEAK_ALLOC.peak_usage_as_mb(),
        duration = ?now.elapsed(),
        "Finished processing images"
    );

    Ok(())
}

async fn process_image(
    gdrive: &GDrive,
    converters: Arc<[Box<dyn Converter>]>,
    out: &GCSBucket,
    res: Result<Image, Error>,
) -> Option<Tree> {
    let image = match res {
        Ok(i) => i,
        Err(err) => {
            error!(%err, "Error retrieving image");
            return None;
        }
    };

    // Download image
    let now = Instant::now();
    let bytes = match gdrive.image_data(&image).await {
        Ok(b) => b,
        Err(err) => {
            error!(%err, image = image.as_value(), "Error downloading image");
            return None;
        }
    };
    info!(image = image.as_value(), duration = ?now.elapsed(), "Downloaded image");

    // Run processing jobs on blocking threads
    let now = Instant::now();

    // Extract metadata from EXIF
    let tree_task = tokio::task::spawn_blocking({
        let bytes = bytes.clone();
        let image = image.clone();
        move || Tree::new(image, bytes)
    });

    // Convert images
    let convert_task = tokio::task::spawn_blocking({
        let bytes = bytes.clone();
        let converters = Arc::clone(&converters);
        let image = image.clone();
        move || {
            for conv in &*converters {
                if conv.supported_mime_types().contains(&&*image.mime) {
                    debug!(image = image.as_value(), "Converting image to webp");
                    return conv.convert(bytes);
                }
            }
            Err(Error::NoAvailableConverter)
        }
    });

    let tree = match tree_task.await.expect("Tree task shouldn't panic") {
        Ok(t) => t,
        Err(err) => {
            error!(%err, image = image.as_value(), "Error extracting metadata from image");
            return None;
        }
    };
    let webp = match convert_task.await.expect("Convert task shouldn't panic") {
        Ok(t) => t,
        Err(err) => {
            error!(%err, image = image.as_value(), "Error converting image to webp");
            return None;
        }
    };

    info!(
        image = image.as_value(),
        tree = tree.as_value(),
        duration = ?now.elapsed(),
        webp.small = webp.small.len(),
        webp.large = webp.large.len(),
        "Finished processing image"
    );

    // Upload images to GCS
    let now = Instant::now();
    match tokio::join!(
        out.upload_image(&image.id, ImageType::Small, webp.small),
        out.upload_image(&image.id, ImageType::Large, webp.large)
    ) {
        (Ok(()), Ok(())) => (),
        (Err(err), _) | (_, Err(err)) => {
            error!(%err, image = image.as_value(), "Error uploading image to GCS");
            return None;
        }
    }

    info!(image = image.as_value(), duration = ?now.elapsed(), "Uploaded images to GCS");

    Some(tree)
}
