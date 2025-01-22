use hyper::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("gcloud error: {0}")]
    Google(#[from] google_apis_common::Error),
    #[error("env var error: {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("config parse error: {0}")]
    Config(#[from] std::num::ParseIntError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing required field: {0}")]
    MissingRequiredField(&'static str),
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("bad status code: {0}")]
    BadStatusCode(StatusCode),

    // Metadata Parsing Errors
    #[error("exif parse error: {0}")]
    ExifParse(#[from] exif::Error),
    #[error("exif missing field: {0}")]
    ExifMissingField(exif::Tag),
    #[error("exif invalid field type")]
    ExifInvalidFieldType,
    #[error("exif utf8 parse error: {0}")]
    ExifUtf8Parse(#[from] std::str::Utf8Error),
    #[error("time parse error: {0}")]
    TimeParse(#[from] time::error::Parse),

    // Converter Errors
    #[error("no available converter")]
    NoAvailableConverter,

    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("invalid pixel layout")]
    InvalidPixelLayout,

    #[error("libheif error: {0}")]
    LibHeif(#[from] libheif_rs::HeifError),
    #[error("libheif missing interleaved RGB plane")]
    LibHeifMissingInterleaved,
    #[error("libheif data length mismatch: {width}x{height}!={length}")]
    LibHeifDataLengthMismatch {
        width: usize,
        height: usize,
        length: usize,
    },

    // Upload errors
    #[error("bad content type: {0}")]
    BadContentType(#[from] mime::FromStrError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
