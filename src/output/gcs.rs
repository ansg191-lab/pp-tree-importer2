use std::{io::Cursor, str::FromStr, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD};
use bytes::{BufMut, Bytes, BytesMut};
use geojson::FeatureCollection;
use google_storage1::{Storage, api::Object};
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use mime::Mime;
use serde_json::Value;
use tracing::{debug, warn};

use crate::{
    config::Config,
    error::Error,
    http::{get_google_default_creds, hyper_client},
    output::{ImageType, Output},
};

const GEOJSON_PATH: &str = "trees.json";
const GEOJSON_CACHE_CONTROL: &str = "no-cache";
const WEBP_MIME: &str = "image/webp";
const DEFAULT_CACHE_CONTROL: &str = "public, max-age=3600";

pub struct GCSBucket {
    hub: Storage<HttpsConnector<HttpConnector>>,
    cfg: Arc<Config>,
}

impl GCSBucket {
    pub async fn new(cfg: Arc<Config>) -> Result<Self, Error> {
        let auth = get_google_default_creds().await?;
        let client = hyper_client();
        let hub = Storage::new(client, auth);
        Ok(Self { hub, cfg })
    }

    async fn get_file(&self, path: &str) -> Result<Option<Object>, Error> {
        let blob = self
            .hub
            .objects()
            .get(&self.cfg.bucket_name, path)
            .doit()
            .await;
        match blob {
            Ok((_, obj)) => Ok(Some(obj)),
            Err(err) => match &err {
                google_apis_common::Error::BadRequest(Value::Object(obj)) => match obj.get("error")
                {
                    Some(Value::Object(obj)) => match obj.get("code") {
                        Some(Value::Number(n)) if n.as_u64().is_some_and(|n| n == 404) => Ok(None),
                        None => Err(err.into()),
                        Some(_) => Err(err.into()),
                    },
                    None => Err(err.into()),
                    Some(_) => Err(err.into()),
                },
                _ => Err(err.into()),
            },
        }
    }

    async fn upload_file_inner(
        &self,
        path: impl Into<String>,
        data: Bytes,
        content_type: impl AsRef<str>,
        cache_control: impl Into<String>,
    ) -> Result<Object, Error> {
        let mime = Mime::from_str(content_type.as_ref())?;
        let obj = Object {
            cache_control: Some(cache_control.into()),
            content_type: Some(mime.essence_str().to_owned()),
            name: Some(path.into()),
            ..Default::default()
        };

        let stream = Cursor::new(data);
        let (_, obj) = self
            .hub
            .objects()
            .insert(obj, &self.cfg.bucket_name)
            .upload(stream, mime)
            .await?;

        Ok(obj)
    }

    #[tracing::instrument(
        skip(self, data),
        fields(bucket = self.cfg.bucket_name),
    )]
    async fn upload_file(
        &self,
        path: String,
        data: Bytes,
        content_type: &str,
        cache_control: String,
    ) -> Result<Object, Error> {
        match self.get_file(&path).await? {
            Some(obj) => {
                // Object exists, check hash
                if let Some(gcs_digest) = &obj.md5_hash {
                    let hash = compute_hash(&data);
                    if &hash == gcs_digest {
                        debug!(
                            path,
                            hash, "Object exists and hash matches, skipping upload"
                        );
                        Ok(obj)
                    } else {
                        debug!(
                            path,
                            hash.local = hash,
                            hash.gcs = gcs_digest,
                            "Object exists but hash doesn't match, re-uploading"
                        );
                        Ok(self
                            .upload_file_inner(&path, data, content_type, cache_control)
                            .await?)
                    }
                } else {
                    warn!(path, "Object exists but has no hash, re-uploading");
                    Ok(self
                        .upload_file_inner(&path, data, content_type, cache_control)
                        .await?)
                }
            }
            None => {
                // Object doesn't exist, upload
                Ok(self
                    .upload_file_inner(&path, data, content_type, cache_control)
                    .await?)
            }
        }
    }
}

impl Output for GCSBucket {
    #[tracing::instrument(skip(self, data), fields(bucket = self.cfg.bucket_name))]
    async fn upload_image(&self, id: &str, tp: ImageType, data: Bytes) -> Result<(), Error> {
        let path = compute_path(id, tp);
        self.upload_file(path, data, WEBP_MIME, DEFAULT_CACHE_CONTROL.to_owned())
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(bucket = self.cfg.bucket_name))]
    async fn upload_geojson(&self, json: &FeatureCollection) -> Result<(), Error> {
        let bytes = BytesMut::new();
        let mut writer = bytes.writer();
        serde_json::to_writer(&mut writer, json)?;
        self.upload_file(
            GEOJSON_PATH.to_owned(),
            writer.into_inner().freeze(),
            mime::APPLICATION_JSON.essence_str(),
            GEOJSON_CACHE_CONTROL.to_owned(),
        )
        .await?;
        Ok(())
    }
}

fn compute_path(id: &str, tp: ImageType) -> String {
    match tp {
        ImageType::Small => format!("{}-small.webp", id),
        ImageType::Large => format!("{}-large.webp", id),
    }
}

fn compute_hash(data: &Bytes) -> String {
    let digest = md5::compute(data);
    BASE64_STANDARD.encode(*digest)
}
