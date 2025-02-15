//! Module to extract EXIF metadata from image

use std::io::Cursor;

use bytes::Bytes;
use chrono::{DateTime, FixedOffset};
use exif::{Exif, In, Reader, Tag, Value};
use geojson::{feature::Id, Feature, Geometry, JsonObject};
use tracing::{debug, error};
use valuable::Valuable;

use crate::{error::Error, image_source::Image};

#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    pub image: Image,
    pub location: Location,
    pub timestamp: DateTime<FixedOffset>,
}

impl Tree {
    #[tracing::instrument(skip_all, fields(image = image.as_value()))]
    pub fn new(image: Image, data: Bytes) -> Result<Self, Error> {
        debug!("Reading EXIF data from image");
        let exif = Reader::new().read_from_container(&mut Cursor::new(data))?;

        let timestamp = get_timestamp(&exif)?;
        let location = Location::from_image(&exif)?;
        Ok(Self {
            image,
            location,
            timestamp,
        })
    }
}

impl From<Tree> for Feature {
    fn from(value: Tree) -> Self {
        let geo = Geometry::from(value.location);
        let timestamp = value.timestamp.to_rfc3339();
        Self {
            bbox: None,
            geometry: Some(geo),
            id: Some(Id::String(value.image.id.clone())),
            properties: Some({
                let mut map = JsonObject::new();
                map.insert("id".to_owned(), value.image.id.into());
                map.insert("timestamp".to_owned(), timestamp.into());
                map.insert("file".to_owned(), value.image.full_path.into());
                map.insert("hash".to_owned(), value.image.digest.into());
                map.insert("tag".to_owned(), value.image.tag.as_str().into());
                map.insert("name".to_owned(), value.image.name.into());
                map
            }),
            foreign_members: None,
        }
    }
}

const _: () = {
    use valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Value, Visit};

    static TREE_FIELDS: &[NamedField<'static>] = &[
        NamedField::new("image"),
        NamedField::new("location"),
        NamedField::new("timestamp"),
    ];
    #[automatically_derived]
    impl Structable for Tree {
        fn definition(&self) -> StructDef<'_> {
            StructDef::new_static("Tree", Fields::Named(TREE_FIELDS))
        }
    }
    #[automatically_derived]
    impl Valuable for Tree {
        fn as_value(&self) -> Value<'_> {
            Value::Structable(self)
        }
        fn visit(&self, visitor: &mut dyn Visit) {
            visitor.visit_named_fields(&NamedValues::new(
                TREE_FIELDS,
                &[
                    Valuable::as_value(&self.image),
                    Valuable::as_value(&self.location),
                    Valuable::as_value(&self.timestamp.to_rfc3339()),
                ],
            ));
        }
    }
};

#[derive(Debug, Copy, Clone, PartialEq, Valuable)]
pub struct Location {
    pub lat: f64,
    pub lon: f64,
}

impl Location {
    pub fn from_image(exif: &Exif) -> Result<Self, Error> {
        let lat = get_gps(exif, Tag::GPSLatitude, Tag::GPSLatitudeRef)?;
        let lon = get_gps(exif, Tag::GPSLongitude, Tag::GPSLongitudeRef)?;
        Ok(Self { lat, lon })
    }
}

impl From<Location> for Geometry {
    fn from(value: Location) -> Self {
        Geometry::new(geojson::Value::Point(vec![value.lon, value.lat]))
    }
}

fn get_gps(exif: &Exif, tag: Tag, tag_ref: Tag) -> Result<f64, Error> {
    let field = exif
        .get_field(tag, In::PRIMARY)
        .ok_or(Error::ExifMissingField(tag))?;
    let field_ref = exif
        .get_field(tag_ref, In::PRIMARY)
        .ok_or(Error::ExifMissingField(tag_ref))?;

    let Value::Rational(rats) = &field.value else {
        error!(tag = %field.tag, value = %field.display_value(), "Invalid field type found");
        return Err(Error::ExifInvalidFieldType);
    };
    let Value::Ascii(ascii) = &field_ref.value else {
        error!(tag = %field_ref.tag, value = %field_ref.display_value(), "Invalid field type found");
        return Err(Error::ExifInvalidFieldType);
    };

    // Compute direction
    let dir = match std::str::from_utf8(&ascii[0])? {
        "S" | "W" => -1.0,
        _ => 1.0,
    };

    // Compute coordinate
    if rats.len() != 3 {
        error!(
            tag = %field.tag,
            value = %field.display_value(),
            len = rats.len(),
            "Invalid field type found"
        );
        return Err(Error::ExifInvalidFieldType);
    }
    let degree = rats[0].to_f64();
    let minute = rats[1].to_f64();
    let second = rats[2].to_f64();
    let value = degree + minute / 60.0 + second / 3600.0;
    Ok(value * dir)
}

fn get_timestamp(exif: &Exif) -> Result<DateTime<FixedOffset>, Error> {
    // Get fields for original datetime
    let datetime_field = exif
        .get_field(Tag::DateTimeOriginal, In::PRIMARY)
        .ok_or(Error::ExifMissingField(Tag::DateTimeOriginal))?;
    let offset_field = exif
        .get_field(Tag::OffsetTimeOriginal, In::PRIMARY)
        .ok_or(Error::ExifMissingField(Tag::OffsetTimeOriginal))?;

    // Check value type
    let Value::Ascii(datetime) = &datetime_field.value else {
        error!(tag = %datetime_field.tag, value = %datetime_field.display_value(), "Invalid field type found");
        return Err(Error::ExifInvalidFieldType);
    };
    let Value::Ascii(offset) = &offset_field.value else {
        error!(tag = %offset_field.tag, value = %offset_field.display_value(), "Invalid field type found");
        return Err(Error::ExifInvalidFieldType);
    };

    // Convert to strings
    let datetime = std::str::from_utf8(&datetime[0])?;
    let offset = std::str::from_utf8(&offset[0])?;

    // Combine for parsing
    let full_datetime = format!("{} {}", datetime, offset);

    // Parse into `OffsetDateTime`
    const FORMAT: &str = "%Y:%m:%d %H:%M:%S %:z";
    Ok(DateTime::parse_from_str(full_datetime.as_str(), FORMAT)?)
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use chrono::{Datelike, Month, Timelike};

    use super::*;

    #[test]
    fn test_get_timestamp() {
        let img = std::fs::read("fixtures/IMG_0406.HEIC").unwrap();
        let exif = Reader::new()
            .read_from_container(&mut Cursor::new(img))
            .unwrap();

        let timestamp = get_timestamp(&exif).unwrap();
        assert_eq!(timestamp.year(), 2025);
        assert_eq!(timestamp.month(), Month::January.number_from_month());
        assert_eq!(timestamp.day(), 18);
        assert_eq!(timestamp.hour(), 12);
        assert_eq!(timestamp.minute(), 14);
        assert_eq!(timestamp.second(), 0);
        assert_eq!(
            *timestamp.offset(),
            FixedOffset::west_opt(8 * 3600).unwrap()
        );
    }

    #[test]
    fn test_location() {
        let img = std::fs::read("fixtures/20250121_065541.jpg").unwrap();
        let exif = Reader::new()
            .read_from_container(&mut Cursor::new(img))
            .unwrap();

        let location = Location::from_image(&exif).unwrap();
        assert_relative_eq!(location.lat, 33.716812, epsilon = 0.00001);
        assert_relative_eq!(location.lon, -117.759817, epsilon = 0.00001);
    }
}
