mod gdrive;
use std::{convert::Infallible, fmt::Display, future::Future, str::FromStr};

use bytes::Bytes;
use futures::Stream;
pub use gdrive::GDrive;

use crate::error::Error;

pub trait ImageSource {
    /// Get a stream of images
    fn images(&self) -> impl Stream<Item = Result<Image, Error>> + Send;

    /// Download image raw bytes
    fn image_data(&self, image: &Image) -> impl Future<Output = Result<Bytes, Error>> + Send;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Image {
    /// Unique image ID
    pub id: String,
    /// File name
    pub name: String,
    /// Tag
    pub tag: Tag,
    /// Full path
    pub full_path: String,
    /// File hash
    pub digest: String,
    /// Mime Type
    pub mime: String,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Tag {
    Unknown,
    Marked,
    Unmarked,
}

impl FromStr for Tag {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "marked" => Ok(Tag::Marked),
            "unmarked" => Ok(Tag::Unmarked),
            _ => Ok(Tag::Unknown),
        }
    }
}

impl Tag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tag::Unknown => "unknown",
            Tag::Marked => "marked",
            Tag::Unmarked => "unmarked",
        }
    }
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
