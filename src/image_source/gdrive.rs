use std::{str::FromStr, sync::Arc};

use bytes::Bytes;
use futures::{Stream, StreamExt};
use google_drive3::{
    api::{File, Scope},
    DriveHub,
};
use http_body_util::BodyExt;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info, trace, warn};
use unwrap_infallible::UnwrapInfallible;
use valuable::Valuable;

use crate::{
    config::Config,
    error::Error,
    http::{get_google_default_creds, hyper_client},
    image_source::{Image, ImageSource, Tag},
    macros::{trys, yield_from},
    ALLOWED_MIME_TYPES,
};

const FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";

/// Google Drive image source
#[derive(Clone)]
pub struct GDrive {
    inner: Arc<GDriveInner>,
}

struct GDriveInner {
    hub: DriveHub<HttpsConnector<HttpConnector>>,
    cfg: Arc<Config>,
}

impl GDrive {
    pub async fn new(cfg: Arc<Config>) -> Result<Self, Error> {
        let auth = get_google_default_creds().await?;
        let client = hyper_client();
        let hub = DriveHub::new(client, auth);
        let inner = Arc::new(GDriveInner { hub, cfg });
        Ok(Self { inner })
    }
}
impl GDriveInner {
    #[tracing::instrument(skip(self))]
    async fn list_files(&self, folder_id: &str) -> Result<Vec<File>, Error> {
        let query = format!("'{folder_id}' in parents and trashed = false");
        let mut page_token = None;
        let mut results = Vec::new();
        trace!("Listing files");

        loop {
            let file_list = self
                .hub
                .files()
                .list()
                .q(&query)
                .add_scope(Scope::Readonly)
                .param(
                    "fields",
                    "nextPageToken, files(id, name, mimeType, sha1Checksum)",
                );
            let (_, file_list) = if let Some(token) = page_token.as_deref() {
                file_list.page_token(token).doit().await
            } else {
                file_list.doit().await
            }?;
            page_token = file_list.next_page_token;
            if let Some(files) = file_list.files {
                results.extend(files);
            }
            if page_token.is_none() {
                break;
            }
        }

        trace!(files = results.len(), "Found {} files", results.len());
        Ok(results)
    }

    async fn get_tags(&self) -> Result<Vec<(Tag, File)>, Error> {
        Ok(self
            .list_files(&self.cfg.gdrive_folder_id)
            .await?
            .into_iter()
            .filter(|f| f.mime_type.as_ref().is_some_and(|m| m == FOLDER_MIME_TYPE))
            .filter_map(|f| Some((Tag::from_str(f.name.as_deref()?).unwrap_infallible(), f)))
            .collect())
    }

    /// Recursively gets the image files in a tag folder.
    ///
    /// # Arguments
    ///
    /// * `folder`: Current search folder (maybe a subfolder via recursive search)
    /// * `tag`: Tag
    /// * `full_path`: Full path from tag root to folder
    #[tracing::instrument(
        skip_all,
        fields(
            tag = tag.as_value(),
            folder.id = %folder.id.as_deref().unwrap_or_default(),
            full_path = %full_path.as_ref(),
        )
    )]
    fn get_images(
        self: Arc<Self>,
        folder: File,
        tag: Tag,
        full_path: impl AsRef<str> + Send + 'static,
    ) -> UnboundedReceiverStream<Result<Image, Error>> {
        trace!("Searching image in folder");
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let full_path = full_path.as_ref();
            let folder_id = trys!(
                tx,
                folder
                    .id
                    .as_deref()
                    .ok_or(Error::MissingRequiredField("id"))
            );
            let files = trys!(tx, self.list_files(folder_id).await);

            for file in files {
                let mime_type = file.mime_type.as_deref().unwrap_or_default();
                if ALLOWED_MIME_TYPES.contains(&mime_type) {
                    // Image
                    let img = trys!(tx, create_image(file, tag, full_path));
                    if tx.send(Ok(img)).is_err() {
                        return;
                    }
                } else if mime_type == FOLDER_MIME_TYPE {
                    // Folder, recurse search
                    let full_path =
                        format!("{full_path}/{}", file.name.as_deref().unwrap_or_default());
                    yield_from!(tx, Arc::clone(&self).get_images(file, tag, full_path));
                } else {
                    // Unknown file
                    warn!(
                        mime_type,
                        folder_id,
                        folder_name = folder.name.as_deref().unwrap_or_default(),
                        full_path,
                        tag = tag.as_value(),
                        "Unsupported file type"
                    );
                }
            }
        });

        UnboundedReceiverStream::new(rx)
    }
}

impl ImageSource for GDrive {
    fn images(&self) -> impl Stream<Item = Result<Image, Error>> + Send {
        let (tx, rx) = mpsc::unbounded_channel();
        let inner = Arc::clone(&self.inner);

        tokio::spawn(async move {
            // Search folder's subfolder tags
            let tags = trys!(tx, inner.get_tags().await);
            for (tag, folder) in tags {
                info!(
                    tag = tag.as_value(),
                    folder_id = folder.id.as_deref().unwrap_or_default(),
                    folder_name = folder.name.as_deref().unwrap_or_default(),
                    "Searching tag folder"
                );
                let full_path = folder.name.clone().expect("All tags have names");
                // Get images from each tag
                yield_from!(tx, Arc::clone(&inner).get_images(folder, tag, full_path));
            }
        });

        UnboundedReceiverStream::new(rx)
    }

    #[tracing::instrument(skip_all, fields(image = image.as_value()))]
    async fn image_data(&self, image: &Image) -> Result<Bytes, Error> {
        debug!("Downloading image");
        let (res, _) = self
            .inner
            .hub
            .files()
            .get(&image.id)
            .add_scope(Scope::Readonly)
            .acknowledge_abuse(true)
            .param("alt", "media")
            .doit()
            .await?;

        if res.status().is_success() {
            Ok(res.into_body().collect().await?.to_bytes())
        } else {
            Err(Error::BadStatusCode(res.status()))
        }
    }
}

fn create_image(file: File, tag: Tag, full_path: &str) -> Result<Image, Error> {
    Ok(Image {
        id: file.id.ok_or(Error::MissingRequiredField("id"))?,
        full_path: format!("{full_path}/{}", file.name.as_deref().unwrap_or_default()),
        name: file.name.unwrap_or_default(),
        tag,
        digest: file.sha1_checksum.unwrap_or_default(),
        mime: file.mime_type.unwrap_or_default(),
    })
}
