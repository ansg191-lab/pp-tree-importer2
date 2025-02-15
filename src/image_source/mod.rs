mod gdrive;
use std::{convert::Infallible, fmt::Display, future::Future, str::FromStr};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::Stream;
pub use gdrive::GDrive;
use valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value, Visit};

use crate::{converter::ImageFormat, error::Error};

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
    pub format: ImageFormat,
    /// Creation time
    pub created: DateTime<Utc>,
    /// Last modified time
    pub modified: DateTime<Utc>,
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

impl Valuable for Tag {
    fn as_value(&self) -> Value<'_> {
        Value::String(self.as_str())
    }
    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value())
    }
}

static IMAGE_FIELDS: &[NamedField<'static>] = &[
    NamedField::new("id"),
    NamedField::new("name"),
    NamedField::new("tag"),
    NamedField::new("full_path"),
    NamedField::new("digest"),
    NamedField::new("format"),
    NamedField::new("created"),
    NamedField::new("modified"),
];
impl Structable for Image {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_static("Image", Fields::Named(IMAGE_FIELDS))
    }
}
impl Valuable for Image {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }
    fn visit(&self, visitor: &mut dyn Visit) {
        visitor.visit_named_fields(&NamedValues::new(
            IMAGE_FIELDS,
            &[
                Valuable::as_value(&self.id),
                Valuable::as_value(&self.name),
                Valuable::as_value(&self.tag),
                Valuable::as_value(&self.full_path),
                Valuable::as_value(&self.digest),
                Valuable::as_value(&self.format),
                Valuable::as_value(&self.created.to_rfc3339()),
                Valuable::as_value(&self.modified.to_rfc3339()),
            ],
        ));
    }
}
